//! Pure-Rust offline ONNX executor and PaddleOCR CTC subtitle recognizer.

use crate::backend::OcrBackend;
use crate::bitmap::{BitmapSubtitleCue, decode_png_to_rgba};
use crate::model;
use anyhow::{Context as _, Result, bail};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const TARGET_LINE_HEIGHT: usize = 48;
const MIN_LINE_WIDTH: f32 = 48.0;
const MAX_LINE_WIDTH: f32 = 960.0;
const MAX_VERTICAL_GAP_PX: usize = 3;
const MIN_LINE_HEIGHT_PX: usize = 6;
const LINE_PAD_PX: usize = 2;

/// Pure-Rust offline OCR backend powered by PaddleOCR CTC ONNX models.
#[derive(Debug, Clone)]
pub struct LocalOcrBackend {
    model: Arc<ParsedModel>,
}

impl LocalOcrBackend {
    /// Loads a cataloged local OCR model from the local cache directory.
    pub fn from_model_id(model_id: &str) -> Result<Self> {
        let path = model::model_file_path(model_id)?;
        if !path.is_file() {
            bail!(
                "Local OCR model '{model_id}' is not downloaded (expected at {}). Download it before running local OCR.",
                path.display()
            );
        }
        Self::from_path(&path)
    }

    /// Loads an ONNX PaddleOCR recognition model from a file on disk.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read ONNX OCR model from {}", path.display()))?;
        Self::from_bytes(&bytes)
    }

    /// Parses an ONNX PaddleOCR recognition model directly from memory.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let parsed = parse_onnx_model(bytes)?;
        Ok(Self {
            model: Arc::new(parsed),
        })
    }

    /// Returns the loaded CTC character vocabulary (`[blank, chars..., " "]`).
    #[cfg(test)]
    pub(crate) fn vocab(&self) -> &[String] {
        &self.model.vocab
    }

    fn recognize_cue_sync(
        model: &ParsedModel,
        png_bytes: &[u8],
        cancel: &CancellationToken,
    ) -> Result<String> {
        anyhow::ensure!(!cancel.is_cancelled(), "OCR cancelled by user");

        let (rgba, w_u32, h_u32) = decode_png_to_rgba(png_bytes)?;
        let width = w_u32 as usize;
        let height = h_u32 as usize;
        if width == 0 || height == 0 {
            return Ok(String::new());
        }

        let bg = estimate_background_rgb(&rgba, width, height);
        let line_boxes = segment_text_lines(&rgba, width, height, bg);
        if line_boxes.is_empty() {
            return Ok(String::new());
        }

        let mut recognized_lines = Vec::with_capacity(line_boxes.len());
        for line_box in line_boxes {
            anyhow::ensure!(!cancel.is_cancelled(), "OCR cancelled by user");
            let input_tensor = preprocess_line_strip(&rgba, width, bg, line_box);
            let output = run_graph(model, input_tensor, cancel)?;
            let decoded = ctc_greedy_decode(&output, &model.vocab)?;
            let trimmed = decoded.trim();
            if !trimmed.is_empty() {
                recognized_lines.push(trimmed.to_string());
            }
        }

        Ok(recognized_lines.join("\n"))
    }
}

impl OcrBackend for LocalOcrBackend {
    fn recognize_cue<'a>(
        &'a self,
        cue: &'a BitmapSubtitleCue,
        _language_hint: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            anyhow::ensure!(!cancel.is_cancelled(), "OCR cancelled by user");
            let model = self.model.clone();
            let png_bytes = cue.png_bytes.clone();
            let token = cancel.clone();
            tokio::task::spawn_blocking(move || {
                Self::recognize_cue_sync(&model, &png_bytes, &token)
            })
            .await
            .context("Local OCR worker task failed")?
        })
    }

    fn recommended_concurrency(&self) -> usize {
        std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(4)
            .clamp(1, 4)
    }
}

// ============================================================================
// Minimal ONNX Protobuf Wire Reader
// ============================================================================

struct ProtoReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ProtoReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn read_varint(&mut self) -> Result<u64> {
        let mut result: u64 = 0;
        let mut shift: u32 = 0;
        for _ in 0..10 {
            let byte = *self
                .data
                .get(self.pos)
                .context("Unexpected EOF while reading protobuf varint")?;
            self.pos += 1;
            result |= u64::from(byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
        }
        bail!("Malformed protobuf varint exceeding 10 bytes")
    }

    fn read_tag(&mut self) -> Result<Option<(u32, u8)>> {
        if self.is_eof() {
            return Ok(None);
        }
        let raw = self.read_varint()?;
        let field_num = u32::try_from(raw >> 3).context("Protobuf field number overflow")?;
        let wire_type = (raw & 0x07) as u8;
        if field_num == 0 {
            bail!("Invalid protobuf field number 0");
        }
        Ok(Some((field_num, wire_type)))
    }

    fn read_bytes(&mut self) -> Result<&'a [u8]> {
        let len_u64 = self.read_varint()?;
        let len = usize::try_from(len_u64).context("Protobuf length exceeds address space")?;
        let end = self
            .pos
            .checked_add(len)
            .context("Protobuf slice offset overflow")?;
        let slice = self
            .data
            .get(self.pos..end)
            .context("Truncated protobuf length-delimited field")?;
        self.pos = end;
        Ok(slice)
    }

    fn read_string(&mut self) -> Result<String> {
        let bytes = self.read_bytes()?;
        let text = std::str::from_utf8(bytes).context("Invalid UTF-8 in protobuf string")?;
        Ok(text.to_string())
    }

    fn read_fixed32(&mut self) -> Result<f32> {
        let end = self
            .pos
            .checked_add(4)
            .context("Protobuf offset overflow")?;
        let bytes = self
            .data
            .get(self.pos..end)
            .context("Truncated protobuf fixed32 field")?;
        self.pos = end;
        Ok(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_fixed64(&mut self) -> Result<f64> {
        let end = self
            .pos
            .checked_add(8)
            .context("Protobuf offset overflow")?;
        let bytes = self
            .data
            .get(self.pos..end)
            .context("Truncated protobuf fixed64 field")?;
        self.pos = end;
        Ok(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_packed_varints(&mut self) -> Result<Vec<u64>> {
        let bytes = self.read_bytes()?;
        let mut sub = ProtoReader::new(bytes);
        let mut out = Vec::new();
        while !sub.is_eof() {
            out.push(sub.read_varint()?);
        }
        Ok(out)
    }

    fn read_packed_f32(&mut self) -> Result<Vec<f32>> {
        let bytes = self.read_bytes()?;
        if !bytes.len().is_multiple_of(4) {
            bail!(
                "Packed float32 field has invalid byte length {}",
                bytes.len()
            );
        }
        Ok(bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&c| f32::from_le_bytes(c))
            .collect())
    }

    fn read_packed_f64(&mut self) -> Result<Vec<f64>> {
        let bytes = self.read_bytes()?;
        if !bytes.len().is_multiple_of(8) {
            bail!(
                "Packed float64 field has invalid byte length {}",
                bytes.len()
            );
        }
        Ok(bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|&c| f64::from_le_bytes(c))
            .collect())
    }

    fn skip_field(&mut self, wire_type: u8) -> Result<()> {
        match wire_type {
            0 => {
                let _ = self.read_varint()?;
                Ok(())
            }
            1 => {
                let end = self
                    .pos
                    .checked_add(8)
                    .context("Protobuf offset overflow")?;
                if end > self.data.len() {
                    bail!("Truncated protobuf 64-bit field");
                }
                self.pos = end;
                Ok(())
            }
            2 => {
                let _ = self.read_bytes()?;
                Ok(())
            }
            5 => {
                let end = self
                    .pos
                    .checked_add(4)
                    .context("Protobuf offset overflow")?;
                if end > self.data.len() {
                    bail!("Truncated protobuf 32-bit field");
                }
                self.pos = end;
                Ok(())
            }
            other => bail!("Unsupported protobuf wire type {other}"),
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedAttribute {
    name: String,
    f: Option<f32>,
    i: Option<i64>,
    s: Option<Vec<u8>>,
    t: Option<TensorValue>,
    floats: Vec<f32>,
    ints: Vec<i64>,
}

#[derive(Debug, Clone)]
struct ParsedNode {
    inputs: Vec<String>,
    outputs: Vec<String>,
    op_type: String,
    attrs: HashMap<String, ParsedAttribute>,
}

impl ParsedNode {
    fn attr(&self, name: &str) -> Option<&ParsedAttribute> {
        self.attrs.get(name)
    }

    fn attr_i(&self, name: &str) -> Option<i64> {
        self.attr(name).and_then(|a| a.i)
    }

    fn attr_f(&self, name: &str) -> Option<f32> {
        self.attr(name).and_then(|a| a.f)
    }

    fn attr_s(&self, name: &str) -> Option<&str> {
        self.attr(name)
            .and_then(|a| a.s.as_deref())
            .and_then(|b| std::str::from_utf8(b).ok())
    }

    fn attr_ints(&self, name: &str) -> Option<&[i64]> {
        self.attr(name)
            .filter(|a| !a.ints.is_empty())
            .map(|a| a.ints.as_slice())
    }

    fn attr_floats(&self, name: &str) -> Option<&[f32]> {
        self.attr(name)
            .filter(|a| !a.floats.is_empty())
            .map(|a| a.floats.as_slice())
    }
}

#[derive(Debug, Clone)]
struct ParsedGraph {
    nodes: Vec<ParsedNode>,
    initializers: HashMap<String, TensorValue>,
    inputs: Vec<String>,
    outputs: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParsedModel {
    input_name: String,
    output_name: String,
    initializers: HashMap<String, TensorValue>,
    nodes: Vec<ParsedNode>,
    last_use: HashMap<String, usize>,
    vocab: Vec<String>,
}

fn build_vocab_from_character_dict(dict: &str) -> Result<Vec<String>> {
    let chars: Vec<String> = dict.lines().map(ToString::to_string).collect();
    if chars.is_empty() {
        bail!("ONNX model 'character' metadata dictionary is empty");
    }
    let mut vocab = Vec::with_capacity(chars.len() + 2);
    // CTC index 0 is reserved for blank, 1..=N map dictionary lines, N+1 is space.
    vocab.push(String::new());
    vocab.extend(chars);
    vocab.push(" ".to_string());
    Ok(vocab)
}

fn parse_onnx_model(bytes: &[u8]) -> Result<ParsedModel> {
    let mut reader = ProtoReader::new(bytes);
    let mut graph: Option<ParsedGraph> = None;
    let mut character_dict: Option<String> = None;

    while let Some((field, wire)) = reader.read_tag()? {
        match (field, wire) {
            (7, 2) => {
                let g_bytes = reader.read_bytes()?;
                graph = Some(parse_graph_proto(g_bytes)?);
            }
            (14, 2) => {
                let entry_bytes = reader.read_bytes()?;
                let (k, v) = parse_string_string_entry(entry_bytes)?;
                if k == "character" {
                    character_dict = Some(v);
                }
            }
            _ => reader.skip_field(wire)?,
        }
    }

    let graph = graph.context("ONNX ModelProto is missing graph (field 7)")?;
    let dict = character_dict
        .context("ONNX ModelProto is missing 'character' metadata dictionary (field 14)")?;
    let vocab = build_vocab_from_character_dict(&dict)?;

    let input_name = graph
        .inputs
        .iter()
        .find(|name| !graph.initializers.contains_key(*name))
        .or_else(|| graph.inputs.first())
        .cloned()
        .context("ONNX graph has no input tensor")?;
    let output_name = graph
        .outputs
        .first()
        .cloned()
        .context("ONNX graph has no output tensor")?;

    let mut last_use: HashMap<String, usize> = HashMap::new();
    for (idx, node) in graph.nodes.iter().enumerate() {
        for inp in &node.inputs {
            if !inp.is_empty() {
                last_use.insert(inp.clone(), idx);
            }
        }
    }
    for out_name in &graph.outputs {
        last_use.remove(out_name);
    }

    Ok(ParsedModel {
        input_name,
        output_name,
        initializers: graph.initializers,
        nodes: graph.nodes,
        last_use,
        vocab,
    })
}

fn parse_string_string_entry(bytes: &[u8]) -> Result<(String, String)> {
    let mut reader = ProtoReader::new(bytes);
    let mut key = String::new();
    let mut value = String::new();
    while let Some((field, wire)) = reader.read_tag()? {
        match (field, wire) {
            (1, 2) => key = reader.read_string()?,
            (2, 2) => value = reader.read_string()?,
            _ => reader.skip_field(wire)?,
        }
    }
    Ok((key, value))
}

fn parse_value_info_name(bytes: &[u8]) -> Result<String> {
    let mut reader = ProtoReader::new(bytes);
    let mut name = String::new();
    while let Some((field, wire)) = reader.read_tag()? {
        match (field, wire) {
            (1, 2) => name = reader.read_string()?,
            _ => reader.skip_field(wire)?,
        }
    }
    Ok(name)
}

fn parse_graph_proto(bytes: &[u8]) -> Result<ParsedGraph> {
    let mut reader = ProtoReader::new(bytes);
    let mut nodes = Vec::new();
    let mut initializers = HashMap::new();
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    while let Some((field, wire)) = reader.read_tag()? {
        match (field, wire) {
            (1, 2) => nodes.push(parse_node_proto(reader.read_bytes()?)?),
            (5, 2) => {
                let (name, tensor) = parse_tensor_proto(reader.read_bytes()?)?;
                initializers.insert(name, tensor);
            }
            (11, 2) => {
                let name = parse_value_info_name(reader.read_bytes()?)?;
                if !name.is_empty() {
                    inputs.push(name);
                }
            }
            (12, 2) => {
                let name = parse_value_info_name(reader.read_bytes()?)?;
                if !name.is_empty() {
                    outputs.push(name);
                }
            }
            _ => reader.skip_field(wire)?,
        }
    }

    Ok(ParsedGraph {
        nodes,
        initializers,
        inputs,
        outputs,
    })
}

fn parse_node_proto(bytes: &[u8]) -> Result<ParsedNode> {
    let mut reader = ProtoReader::new(bytes);
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut op_type = String::new();
    let mut attrs = HashMap::new();

    while let Some((field, wire)) = reader.read_tag()? {
        match (field, wire) {
            (1, 2) => inputs.push(reader.read_string()?),
            (2, 2) => outputs.push(reader.read_string()?),
            (4, 2) => op_type = reader.read_string()?,
            (5, 2) => {
                let attr = parse_attribute_proto(reader.read_bytes()?)?;
                attrs.insert(attr.name.clone(), attr);
            }
            _ => reader.skip_field(wire)?,
        }
    }

    Ok(ParsedNode {
        inputs,
        outputs,
        op_type,
        attrs,
    })
}

fn parse_attribute_proto(bytes: &[u8]) -> Result<ParsedAttribute> {
    let mut reader = ProtoReader::new(bytes);
    let mut name = String::new();
    let mut f = None;
    let mut i = None;
    let mut s = None;
    let mut t = None;
    let mut floats = Vec::new();
    let mut ints = Vec::new();

    while let Some((field, wire)) = reader.read_tag()? {
        match (field, wire) {
            (1, 2) => name = reader.read_string()?,
            (2, 5) => f = Some(reader.read_fixed32()?),
            (3, 0) => i = Some(reader.read_varint()? as i64),
            (4, 2) => s = Some(reader.read_bytes()?.to_vec()),
            (5, 2) => {
                let (_, tensor) = parse_tensor_proto(reader.read_bytes()?)?;
                t = Some(tensor);
            }
            (7, 5) => floats.push(reader.read_fixed32()?),
            (7, 2) => floats.extend(reader.read_packed_f32()?),
            (8, 0) => ints.push(reader.read_varint()? as i64),
            (8, 2) => ints.extend(reader.read_packed_varints()?.into_iter().map(|v| v as i64)),
            (20, 0) => {
                let _attr_type = reader.read_varint()?;
            }
            _ => reader.skip_field(wire)?,
        }
    }

    Ok(ParsedAttribute {
        name,
        f,
        i,
        s,
        t,
        floats,
        ints,
    })
}

fn parse_tensor_proto(bytes: &[u8]) -> Result<(String, TensorValue)> {
    let mut reader = ProtoReader::new(bytes);
    let mut dims: Vec<i64> = Vec::new();
    let mut data_type: i32 = 1;
    let mut float_data: Vec<f32> = Vec::new();
    let mut int32_data: Vec<i32> = Vec::new();
    let mut int64_data: Vec<i64> = Vec::new();
    let mut double_data: Vec<f64> = Vec::new();
    let mut name = String::new();
    let mut raw_data: Vec<u8> = Vec::new();
    let mut data_location: i64 = 0;

    while let Some((field, wire)) = reader.read_tag()? {
        match (field, wire) {
            (1, 0) => dims.push(reader.read_varint()? as i64),
            (1, 2) => dims.extend(reader.read_packed_varints()?.into_iter().map(|v| v as i64)),
            (2, 0) => data_type = (reader.read_varint()? as i64) as i32,
            (4, 5) => float_data.push(reader.read_fixed32()?),
            (4, 2) => float_data.extend(reader.read_packed_f32()?),
            (5, 0) => int32_data.push((reader.read_varint()? as i64) as i32),
            (5, 2) => int32_data.extend(
                reader
                    .read_packed_varints()?
                    .into_iter()
                    .map(|v| (v as i64) as i32),
            ),
            (7, 0) => int64_data.push(reader.read_varint()? as i64),
            (7, 2) => {
                int64_data.extend(reader.read_packed_varints()?.into_iter().map(|v| v as i64))
            }
            (8, 2) => name = reader.read_string()?,
            (9, 2) => raw_data = reader.read_bytes()?.to_vec(),
            (10, 1) => double_data.push(reader.read_fixed64()?),
            (10, 2) => double_data.extend(reader.read_packed_f64()?),
            (14, 0) => data_location = reader.read_varint()? as i64,
            _ => reader.skip_field(wire)?,
        }
    }

    if data_location == 1 {
        bail!("External ONNX tensor data for '{name}' is not supported");
    }

    let mut shape = Vec::with_capacity(dims.len());
    let mut numel: usize = 1;
    for d in dims {
        let du = usize::try_from(d)
            .with_context(|| format!("Negative tensor dimension {d} in '{name}'"))?;
        numel = numel
            .checked_mul(du)
            .with_context(|| format!("Tensor element count overflow in '{name}'"))?;
        shape.push(du);
    }

    let tensor = match data_type {
        // FLOAT32
        1 => {
            let values = if !raw_data.is_empty() {
                decode_raw_f32(&raw_data, numel, &name)?
            } else if !float_data.is_empty() {
                anyhow::ensure!(
                    float_data.len() == numel,
                    "Tensor '{name}' float_data len {} != expected {numel}",
                    float_data.len()
                );
                float_data
            } else if numel == 0 {
                Vec::new()
            } else {
                bail!("Tensor '{name}' missing float32 payload");
            };
            TensorValue::F32(values, shape)
        }
        // INT32
        6 => {
            let values = if !raw_data.is_empty() {
                decode_raw_i32(&raw_data, numel, &name)?
            } else if !int32_data.is_empty() {
                anyhow::ensure!(
                    int32_data.len() == numel,
                    "Tensor '{name}' int32_data len {} != expected {numel}",
                    int32_data.len()
                );
                int32_data.into_iter().map(i64::from).collect()
            } else if numel == 0 {
                Vec::new()
            } else {
                bail!("Tensor '{name}' missing int32 payload");
            };
            TensorValue::I64(values, shape)
        }
        // INT64
        7 => {
            let values = if !raw_data.is_empty() {
                decode_raw_i64(&raw_data, numel, &name)?
            } else if !int64_data.is_empty() {
                anyhow::ensure!(
                    int64_data.len() == numel,
                    "Tensor '{name}' int64_data len {} != expected {numel}",
                    int64_data.len()
                );
                int64_data
            } else if numel == 0 {
                Vec::new()
            } else {
                bail!("Tensor '{name}' missing int64 payload");
            };
            TensorValue::I64(values, shape)
        }
        // BOOL
        9 => {
            let values = if !raw_data.is_empty() {
                anyhow::ensure!(
                    raw_data.len() == numel,
                    "Tensor '{name}' bool raw_data len {} != expected {numel}",
                    raw_data.len()
                );
                raw_data.into_iter().map(|b| i64::from(b != 0)).collect()
            } else if !int32_data.is_empty() {
                anyhow::ensure!(
                    int32_data.len() == numel,
                    "Tensor '{name}' bool int32_data len {} != expected {numel}",
                    int32_data.len()
                );
                int32_data.into_iter().map(|v| i64::from(v != 0)).collect()
            } else if numel == 0 {
                Vec::new()
            } else {
                bail!("Tensor '{name}' missing bool payload");
            };
            TensorValue::I64(values, shape)
        }
        // FLOAT16
        10 => {
            let expected = numel.checked_mul(2).context("Tensor byte size overflow")?;
            anyhow::ensure!(
                raw_data.len() == expected,
                "Tensor '{name}' float16 raw_data len {} != expected {expected}",
                raw_data.len()
            );
            let values = raw_data
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&c| f16_to_f32(u16::from_le_bytes(c)))
                .collect();
            TensorValue::F32(values, shape)
        }
        // DOUBLE
        11 => {
            let values = if !raw_data.is_empty() {
                let expected = numel.checked_mul(8).context("Tensor byte size overflow")?;
                anyhow::ensure!(
                    raw_data.len() == expected,
                    "Tensor '{name}' double raw_data len {} != expected {expected}",
                    raw_data.len()
                );
                raw_data
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|&c| f64::from_le_bytes(c) as f32)
                    .collect()
            } else if !double_data.is_empty() {
                anyhow::ensure!(
                    double_data.len() == numel,
                    "Tensor '{name}' double_data len {} != expected {numel}",
                    double_data.len()
                );
                double_data.into_iter().map(|d| d as f32).collect()
            } else if numel == 0 {
                Vec::new()
            } else {
                bail!("Tensor '{name}' missing float64 payload");
            };
            TensorValue::F32(values, shape)
        }
        other => bail!("Unsupported ONNX TensorProto data_type {other} for '{name}'"),
    };

    Ok((name, tensor))
}

fn decode_raw_f32(raw: &[u8], numel: usize, name: &str) -> Result<Vec<f32>> {
    let expected = numel.checked_mul(4).context("Tensor byte size overflow")?;
    anyhow::ensure!(
        raw.len() == expected,
        "Tensor '{name}' float32 raw_data len {} != expected {expected}",
        raw.len()
    );
    Ok(raw
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&c| f32::from_le_bytes(c))
        .collect())
}

fn decode_raw_i32(raw: &[u8], numel: usize, name: &str) -> Result<Vec<i64>> {
    let expected = numel.checked_mul(4).context("Tensor byte size overflow")?;
    anyhow::ensure!(
        raw.len() == expected,
        "Tensor '{name}' int32 raw_data len {} != expected {expected}",
        raw.len()
    );
    Ok(raw
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&c| i64::from(i32::from_le_bytes(c)))
        .collect())
}

fn decode_raw_i64(raw: &[u8], numel: usize, name: &str) -> Result<Vec<i64>> {
    let expected = numel.checked_mul(8).context("Tensor byte size overflow")?;
    anyhow::ensure!(
        raw.len() == expected,
        "Tensor '{name}' int64 raw_data len {} != expected {expected}",
        raw.len()
    );
    Ok(raw
        .as_chunks::<8>()
        .0
        .iter()
        .map(|&c| i64::from_le_bytes(c))
        .collect())
}

fn f16_to_f32(bits: u16) -> f32 {
    let sign = u32::from(bits & 0x8000) << 16;
    let exp = u32::from((bits >> 10) & 0x1F);
    let mant = u32::from(bits & 0x03FF);
    let out_bits = if exp == 0 {
        if mant == 0 {
            sign
        } else {
            let mut m = mant;
            let mut e = 127 - 14;
            while (m & 0x0400) == 0 {
                m <<= 1;
                e -= 1;
            }
            m &= 0x03FF;
            sign | (e << 23) | (m << 13)
        }
    } else if exp == 31 {
        sign | 0x7F80_0000 | (mant << 13)
    } else {
        sign | ((exp + 127 - 15) << 23) | (mant << 13)
    };
    f32::from_bits(out_bits)
}

// ============================================================================
// Tensor Runtime & Operator Implementations
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum TensorValue {
    F32(Vec<f32>, Vec<usize>),
    I64(Vec<i64>, Vec<usize>),
}

impl TensorValue {
    fn shape(&self) -> &[usize] {
        match self {
            Self::F32(_, s) | Self::I64(_, s) => s,
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::F32(v, _) => v.len(),
            Self::I64(v, _) => v.len(),
        }
    }

    fn as_f32(&self) -> Result<(&[f32], &[usize])> {
        match self {
            Self::F32(v, s) => Ok((v, s)),
            Self::I64(_, _) => bail!("Expected float32 tensor, found int64 tensor"),
        }
    }

    fn as_i64(&self) -> Result<(&[i64], &[usize])> {
        match self {
            Self::I64(v, s) => Ok((v, s)),
            Self::F32(_, _) => bail!("Expected int64 tensor, found float32 tensor"),
        }
    }

    fn to_f32_vec(&self) -> Vec<f32> {
        match self {
            Self::F32(v, _) => v.clone(),
            Self::I64(v, _) => v.iter().map(|&x| x as f32).collect(),
        }
    }

    fn with_shape(&self, new_shape: Vec<usize>) -> Self {
        match self {
            Self::F32(v, _) => Self::F32(v.clone(), new_shape),
            Self::I64(v, _) => Self::I64(v.clone(), new_shape),
        }
    }
}

fn compute_strides(shape: &[usize]) -> Vec<usize> {
    let mut strides = vec![1usize; shape.len()];
    for i in (0..shape.len()).rev().skip(1) {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}

fn normalize_axis(axis: i64, rank: usize) -> Result<usize> {
    let rank_i = i64::try_from(rank).context("Tensor rank overflow")?;
    let norm = if axis < 0 { axis + rank_i } else { axis };
    if norm < 0 || norm >= rank_i {
        bail!("Axis {axis} is out of bounds for rank {rank}");
    }
    Ok(norm as usize)
}

fn broadcast_shapes(a: &[usize], b: &[usize]) -> Result<(Vec<usize>, Vec<usize>, Vec<usize>)> {
    let out_rank = a.len().max(b.len());
    let a_strides = compute_strides(a);
    let b_strides = compute_strides(b);
    let mut out_shape = vec![0usize; out_rank];
    let mut a_bcast = vec![0usize; out_rank];
    let mut b_bcast = vec![0usize; out_rank];

    for k in 0..out_rank {
        let a_idx = k.checked_sub(out_rank - a.len());
        let b_idx = k.checked_sub(out_rank - b.len());
        let da = a_idx.map_or(1, |i| a[i]);
        let db = b_idx.map_or(1, |i| b[i]);

        let out_d = if da == db {
            da
        } else if da == 1 {
            db
        } else if db == 1 {
            da
        } else {
            bail!("Incompatible broadcast shapes {a:?} and {b:?} at axis {k}");
        };
        out_shape[k] = out_d;
        a_bcast[k] = match a_idx {
            Some(i) if da == out_d && da != 1 => a_strides[i],
            _ => 0,
        };
        b_bcast[k] = match b_idx {
            Some(i) if db == out_d && db != 1 => b_strides[i],
            _ => 0,
        };
    }

    Ok((out_shape, a_bcast, b_bcast))
}

#[derive(Clone, Copy)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

fn eval_binary(a: &TensorValue, b: &TensorValue, op: BinaryOp) -> Result<TensorValue> {
    if let (TensorValue::I64(av, ashape), TensorValue::I64(bv, bshape)) = (a, b) {
        return eval_binary_i64(av, ashape, bv, bshape, op);
    }
    let a_vec = a.to_f32_vec();
    let b_vec = b.to_f32_vec();
    eval_binary_f32(&a_vec, a.shape(), &b_vec, b.shape(), op)
}

fn apply_f32_op(x: f32, y: f32, op: BinaryOp) -> f32 {
    match op {
        BinaryOp::Add => x + y,
        BinaryOp::Sub => x - y,
        BinaryOp::Mul => x * y,
        BinaryOp::Div => x / y,
        BinaryOp::Pow => x.powf(y),
    }
}

fn eval_binary_f32(
    a: &[f32],
    a_shape: &[usize],
    b: &[f32],
    b_shape: &[usize],
    op: BinaryOp,
) -> Result<TensorValue> {
    if a_shape == b_shape {
        let out = a
            .iter()
            .zip(b.iter())
            .map(|(&x, &y)| apply_f32_op(x, y, op))
            .collect();
        return Ok(TensorValue::F32(out, a_shape.to_vec()));
    }
    if b.len() == 1 {
        let y = b[0];
        let out = a.iter().map(|&x| apply_f32_op(x, y, op)).collect();
        return Ok(TensorValue::F32(out, a_shape.to_vec()));
    }
    if a.len() == 1 {
        let x = a[0];
        let out = b.iter().map(|&y| apply_f32_op(x, y, op)).collect();
        return Ok(TensorValue::F32(out, b_shape.to_vec()));
    }

    let (out_shape, a_strides, b_strides) = broadcast_shapes(a_shape, b_shape)?;
    let total = out_shape.iter().product();
    if total == 0 {
        return Ok(TensorValue::F32(Vec::new(), out_shape));
    }
    let rank = out_shape.len();
    let mut coords = vec![0usize; rank];
    let mut out = Vec::with_capacity(total);
    let mut a_off = 0usize;
    let mut b_off = 0usize;

    for _ in 0..total {
        let x = *a.get(a_off).context("Broadcast LHS index out of bounds")?;
        let y = *b.get(b_off).context("Broadcast RHS index out of bounds")?;
        out.push(apply_f32_op(x, y, op));

        for axis in (0..rank).rev() {
            coords[axis] += 1;
            a_off += a_strides[axis];
            b_off += b_strides[axis];
            if coords[axis] < out_shape[axis] {
                break;
            }
            a_off -= coords[axis] * a_strides[axis];
            b_off -= coords[axis] * b_strides[axis];
            coords[axis] = 0;
        }
    }

    Ok(TensorValue::F32(out, out_shape))
}

fn apply_i64_op(x: i64, y: i64, op: BinaryOp) -> Result<i64> {
    match op {
        BinaryOp::Add => Ok(x.wrapping_add(y)),
        BinaryOp::Sub => Ok(x.wrapping_sub(y)),
        BinaryOp::Mul => Ok(x.wrapping_mul(y)),
        BinaryOp::Div => {
            if y == 0 {
                bail!("Integer division by zero in ONNX graph");
            }
            Ok(x.wrapping_div(y))
        }
        BinaryOp::Pow => {
            let exp = u32::try_from(y).context("Negative or overflowing integer Pow exponent")?;
            Ok(x.wrapping_pow(exp))
        }
    }
}

fn eval_binary_i64(
    a: &[i64],
    a_shape: &[usize],
    b: &[i64],
    b_shape: &[usize],
    op: BinaryOp,
) -> Result<TensorValue> {
    let (out_shape, a_strides, b_strides) = broadcast_shapes(a_shape, b_shape)?;
    let total: usize = out_shape.iter().product();
    if total == 0 {
        return Ok(TensorValue::I64(Vec::new(), out_shape));
    }
    let rank = out_shape.len();
    let mut coords = vec![0usize; rank];
    let mut out = Vec::with_capacity(total);
    let mut a_off = 0usize;
    let mut b_off = 0usize;

    for _ in 0..total {
        let x = *a.get(a_off).context("Broadcast LHS index out of bounds")?;
        let y = *b.get(b_off).context("Broadcast RHS index out of bounds")?;
        out.push(apply_i64_op(x, y, op)?);

        for axis in (0..rank).rev() {
            coords[axis] += 1;
            a_off += a_strides[axis];
            b_off += b_strides[axis];
            if coords[axis] < out_shape[axis] {
                break;
            }
            a_off -= coords[axis] * a_strides[axis];
            b_off -= coords[axis] * b_strides[axis];
            coords[axis] = 0;
        }
    }

    Ok(TensorValue::I64(out, out_shape))
}

fn map_f32(t: &TensorValue, f: impl Fn(f32) -> f32) -> Result<TensorValue> {
    let (data, shape) = t.as_f32()?;
    Ok(TensorValue::F32(
        data.iter().map(|&x| f(x)).collect(),
        shape.to_vec(),
    ))
}

fn matmul_2d(a: &[f32], b: &[f32], out: &mut [f32], m: usize, k: usize, n: usize) -> Result<()> {
    anyhow::ensure!(a.len() >= m * k && b.len() >= k * n && out.len() >= m * n);
    out[..m * n].fill(0.0);
    for i in 0..m {
        let out_row = &mut out[i * n..(i + 1) * n];
        let a_row = &a[i * k..(i + 1) * k];
        for (p, &a_val) in a_row.iter().enumerate() {
            if a_val == 0.0 {
                continue;
            }
            let b_row = &b[p * n..(p + 1) * n];
            for (dst, &b_val) in out_row.iter_mut().zip(b_row.iter()) {
                *dst += a_val * b_val;
            }
        }
    }
    Ok(())
}

fn resolve_spatial_pads(
    h_in: usize,
    w_in: usize,
    eff_kh: usize,
    eff_kw: usize,
    strides: [usize; 2],
    node: &ParsedNode,
) -> Result<[usize; 4]> {
    if let Some(auto_pad) = node.attr_s("auto_pad") {
        match auto_pad {
            "VALID" => return Ok([0, 0, 0, 0]),
            "SAME_UPPER" | "SAME_LOWER" => {
                let out_h = h_in.div_ceil(strides[0]);
                let out_w = w_in.div_ceil(strides[1]);
                let pad_h = ((out_h.saturating_sub(1)) * strides[0] + eff_kh).saturating_sub(h_in);
                let pad_w = ((out_w.saturating_sub(1)) * strides[1] + eff_kw).saturating_sub(w_in);
                let (pt, pb) = if auto_pad == "SAME_UPPER" {
                    (pad_h / 2, pad_h - pad_h / 2)
                } else {
                    (pad_h - pad_h / 2, pad_h / 2)
                };
                let (pl, pr) = if auto_pad == "SAME_UPPER" {
                    (pad_w / 2, pad_w - pad_w / 2)
                } else {
                    (pad_w - pad_w / 2, pad_w / 2)
                };
                return Ok([pt, pl, pb, pr]);
            }
            "NOTSET" | "" => {}
            other => bail!("Unsupported auto_pad '{other}'"),
        }
    }

    let raw = node.attr_ints("pads").unwrap_or(&[]);
    if raw.is_empty() {
        return Ok([0, 0, 0, 0]);
    }
    anyhow::ensure!(raw.len() == 4, "Expected 4 pad values, got {}", raw.len());
    Ok([
        usize::try_from(raw[0]).context("Negative pad_top")?,
        usize::try_from(raw[1]).context("Negative pad_left")?,
        usize::try_from(raw[2]).context("Negative pad_bottom")?,
        usize::try_from(raw[3]).context("Negative pad_right")?,
    ])
}

fn read_pair_attr(node: &ParsedNode, name: &str, default: [usize; 2]) -> Result<[usize; 2]> {
    let Some(vals) = node.attr_ints(name) else {
        return Ok(default);
    };
    anyhow::ensure!(
        vals.len() == 2,
        "Expected 2 integers for attribute '{name}', got {}",
        vals.len()
    );
    Ok([
        usize::try_from(vals[0]).with_context(|| format!("Negative value in '{name}'"))?,
        usize::try_from(vals[1]).with_context(|| format!("Negative value in '{name}'"))?,
    ])
}

fn eval_conv(
    x_val: &TensorValue,
    w_val: &TensorValue,
    bias_val: Option<&TensorValue>,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let (x, xs) = x_val.as_f32()?;
    let (w, ws) = w_val.as_f32()?;
    anyhow::ensure!(
        xs.len() == 4 && ws.len() == 4,
        "Conv expects 4D NCHW input and OIHW weight, got {xs:?} and {ws:?}"
    );
    let (n, c_in, h_in, w_in) = (xs[0], xs[1], xs[2], xs[3]);
    let (c_out, c_in_g, kh, kw) = (ws[0], ws[1], ws[2], ws[3]);
    anyhow::ensure!(
        c_in > 0 && c_out > 0 && kh > 0 && kw > 0,
        "Conv dimensions must be > 0, got X={xs:?}, W={ws:?}"
    );
    let group = usize::try_from(node.attr_i("group").unwrap_or(1)).context("Invalid Conv group")?;
    anyhow::ensure!(group >= 1, "Conv group must be >= 1");
    anyhow::ensure!(
        c_in == c_in_g * group && c_out.is_multiple_of(group),
        "Conv channel/group mismatch: c_in={c_in}, c_in_g={c_in_g}, c_out={c_out}, group={group}"
    );

    let strides = read_pair_attr(node, "strides", [1, 1])?;
    let dilations = read_pair_attr(node, "dilations", [1, 1])?;
    anyhow::ensure!(
        strides[0] > 0 && strides[1] > 0 && dilations[0] > 0 && dilations[1] > 0,
        "Conv strides and dilations must be > 0"
    );
    let eff_kh = (kh - 1) * dilations[0] + 1;
    let eff_kw = (kw - 1) * dilations[1] + 1;
    let pads = resolve_spatial_pads(h_in, w_in, eff_kh, eff_kw, strides, node)?;
    let h_pad = h_in + pads[0] + pads[2];
    let w_pad = w_in + pads[1] + pads[3];
    anyhow::ensure!(
        h_pad >= eff_kh && w_pad >= eff_kw,
        "Conv kernel ({eff_kh}x{eff_kw}) exceeds padded input ({h_pad}x{w_pad})"
    );
    let out_h = (h_pad - eff_kh) / strides[0] + 1;
    let out_w = (w_pad - eff_kw) / strides[1] + 1;
    let out_hw = out_h * out_w;
    let in_hw = h_in * w_in;
    let c_out_g = c_out / group;

    let mut out = vec![0.0f32; n * c_out * out_hw];

    // Fast path 1: depthwise separable conv (group == c_in == c_out)
    if group == c_in && c_out == c_in && c_in_g == 1 {
        for batch in 0..n {
            for c in 0..c_out {
                let x_ch = &x[(batch * c_in + c) * in_hw..(batch * c_in + c + 1) * in_hw];
                let w_ch = &w[c * kh * kw..(c + 1) * kh * kw];
                let out_ch =
                    &mut out[(batch * c_out + c) * out_hw..(batch * c_out + c + 1) * out_hw];
                for ky in 0..kh {
                    for kx in 0..kw {
                        let weight = w_ch[ky * kw + kx];
                        if weight == 0.0 {
                            continue;
                        }
                        for oy in 0..out_h {
                            let iy_signed =
                                (oy * strides[0] + ky * dilations[0]) as isize - pads[0] as isize;
                            if iy_signed < 0 || iy_signed >= h_in as isize {
                                continue;
                            }
                            let iy = iy_signed as usize;
                            let x_row = &x_ch[iy * w_in..(iy + 1) * w_in];
                            let o_row = &mut out_ch[oy * out_w..(oy + 1) * out_w];
                            for (ox, dst) in o_row.iter_mut().enumerate() {
                                let ix_signed = (ox * strides[1] + kx * dilations[1]) as isize
                                    - pads[1] as isize;
                                if ix_signed >= 0 && ix_signed < w_in as isize {
                                    *dst += weight * x_row[ix_signed as usize];
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if kh == 1 && kw == 1 && pads == [0, 0, 0, 0] && strides == [1, 1] && group == 1 {
        // Fast path 2: 1x1 pointwise convolution directly via matmul
        for batch in 0..n {
            let x_b = &x[batch * c_in * in_hw..(batch + 1) * c_in * in_hw];
            let out_b = &mut out[batch * c_out * out_hw..(batch + 1) * c_out * out_hw];
            matmul_2d(w, x_b, out_b, c_out, c_in, out_hw)?;
        }
    } else {
        // General im2col + matmul per batch & group
        let col_k = c_in_g * kh * kw;
        let mut col = vec![0.0f32; col_k * out_hw];
        for batch in 0..n {
            for g in 0..group {
                let x_g_base = (batch * c_in + g * c_in_g) * in_hw;
                for ic in 0..c_in_g {
                    let x_ch = &x[x_g_base + ic * in_hw..x_g_base + (ic + 1) * in_hw];
                    for ky in 0..kh {
                        for kx in 0..kw {
                            let row_idx = (ic * kh + ky) * kw + kx;
                            let col_row = &mut col[row_idx * out_hw..(row_idx + 1) * out_hw];
                            for oy in 0..out_h {
                                let iy_signed = (oy * strides[0] + ky * dilations[0]) as isize
                                    - pads[0] as isize;
                                let dst_row = &mut col_row[oy * out_w..(oy + 1) * out_w];
                                if iy_signed < 0 || iy_signed >= h_in as isize {
                                    dst_row.fill(0.0);
                                    continue;
                                }
                                let x_row = &x_ch
                                    [(iy_signed as usize) * w_in..(iy_signed as usize + 1) * w_in];
                                for (ox, dst) in dst_row.iter_mut().enumerate() {
                                    let ix_signed = (ox * strides[1] + kx * dilations[1]) as isize
                                        - pads[1] as isize;
                                    *dst = if ix_signed >= 0 && ix_signed < w_in as isize {
                                        x_row[ix_signed as usize]
                                    } else {
                                        0.0
                                    };
                                }
                            }
                        }
                    }
                }
                let w_g = &w[g * c_out_g * col_k..(g + 1) * c_out_g * col_k];
                let out_g_base = (batch * c_out + g * c_out_g) * out_hw;
                let out_g = &mut out[out_g_base..out_g_base + c_out_g * out_hw];
                matmul_2d(w_g, &col, out_g, c_out_g, col_k, out_hw)?;
            }
        }
    }

    if let Some(bias_t) = bias_val {
        let (bias, _) = bias_t.as_f32()?;
        anyhow::ensure!(
            bias.len() == c_out,
            "Conv bias length {} != c_out {c_out}",
            bias.len()
        );
        for batch in 0..n {
            for (c, &b_val) in bias.iter().enumerate() {
                let start = (batch * c_out + c) * out_hw;
                for dst in &mut out[start..start + out_hw] {
                    *dst += b_val;
                }
            }
        }
    }

    Ok(TensorValue::F32(out, vec![n, c_out, out_h, out_w]))
}

fn eval_batch_norm(
    x_val: &TensorValue,
    scale_val: &TensorValue,
    bias_val: &TensorValue,
    mean_val: &TensorValue,
    var_val: &TensorValue,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let (x, xs) = x_val.as_f32()?;
    let (scale, _) = scale_val.as_f32()?;
    let (bias, _) = bias_val.as_f32()?;
    let (mean, _) = mean_val.as_f32()?;
    let (var, _) = var_val.as_f32()?;
    let eps = node.attr_f("epsilon").unwrap_or(1e-5);
    anyhow::ensure!(
        !xs.is_empty(),
        "BatchNormalization does not accept 0D scalar"
    );

    let (outer, channels, inner) = match xs.len() {
        1 => (1usize, xs[0], 1usize),
        _ => (xs[0], xs[1], xs[2..].iter().product()),
    };
    anyhow::ensure!(
        scale.len() == channels
            && bias.len() == channels
            && mean.len() == channels
            && var.len() == channels,
        "BatchNormalization channel count {channels} mismatch"
    );

    let mut inv_std = Vec::with_capacity(channels);
    let mut shift = Vec::with_capacity(channels);
    for c in 0..channels {
        let s = scale[c] / (var[c] + eps).sqrt();
        inv_std.push(s);
        shift.push(bias[c] - mean[c] * s);
    }

    let mut out = vec![0.0f32; x.len()];
    for o in 0..outer {
        for c in 0..channels {
            let s = inv_std[c];
            let sh = shift[c];
            let base = (o * channels + c) * inner;
            for (dst, &src) in out[base..base + inner]
                .iter_mut()
                .zip(&x[base..base + inner])
            {
                *dst = src * s + sh;
            }
        }
    }

    Ok(TensorValue::F32(out, xs.to_vec()))
}

#[derive(Clone, Copy)]
enum PoolKind {
    Average,
    Max,
}

fn eval_pool2d(x_val: &TensorValue, node: &ParsedNode, kind: PoolKind) -> Result<TensorValue> {
    let (x, xs) = x_val.as_f32()?;
    anyhow::ensure!(xs.len() == 4, "Pool2d expects 4D NCHW input, got {xs:?}");
    let (n, c, h_in, w_in) = (xs[0], xs[1], xs[2], xs[3]);

    let kernel = read_pair_attr(node, "kernel_shape", [1, 1])?;
    let strides = read_pair_attr(node, "strides", [1, 1])?;
    let dilations = read_pair_attr(node, "dilations", [1, 1])?;
    anyhow::ensure!(
        kernel[0] > 0 && kernel[1] > 0 && strides[0] > 0 && strides[1] > 0,
        "Pool2d kernel and strides must be > 0"
    );
    let eff_kh = (kernel[0] - 1) * dilations[0] + 1;
    let eff_kw = (kernel[1] - 1) * dilations[1] + 1;
    let pads = resolve_spatial_pads(h_in, w_in, eff_kh, eff_kw, strides, node)?;
    let ceil_mode = node.attr_i("ceil_mode").unwrap_or(0) != 0;
    let count_include_pad = node.attr_i("count_include_pad").unwrap_or(0) != 0;

    let h_pad = h_in + pads[0] + pads[2];
    let w_pad = w_in + pads[1] + pads[3];
    anyhow::ensure!(h_pad >= eff_kh && w_pad >= eff_kw);

    let out_h = if ceil_mode {
        (h_pad - eff_kh).div_ceil(strides[0]) + 1
    } else {
        (h_pad - eff_kh) / strides[0] + 1
    };
    let out_w = if ceil_mode {
        (w_pad - eff_kw).div_ceil(strides[1]) + 1
    } else {
        (w_pad - eff_kw) / strides[1] + 1
    };

    let in_hw = h_in * w_in;
    let out_hw = out_h * out_w;
    let mut out = vec![0.0f32; n * c * out_hw];

    for batch in 0..n {
        for ch in 0..c {
            let x_ch = &x[(batch * c + ch) * in_hw..(batch * c + ch + 1) * in_hw];
            let o_ch = &mut out[(batch * c + ch) * out_hw..(batch * c + ch + 1) * out_hw];
            for oy in 0..out_h {
                for ox in 0..out_w {
                    match kind {
                        PoolKind::Average => {
                            let mut sum = 0.0f32;
                            let mut count = 0usize;
                            for ky in 0..kernel[0] {
                                let iy = (oy * strides[0] + ky) as isize - pads[0] as isize;
                                if iy < 0 || iy >= h_in as isize {
                                    continue;
                                }
                                let row = &x_ch[(iy as usize) * w_in..(iy as usize + 1) * w_in];
                                for kx in 0..kernel[1] {
                                    let ix = (ox * strides[1] + kx) as isize - pads[1] as isize;
                                    if ix >= 0 && ix < w_in as isize {
                                        sum += row[ix as usize];
                                        count += 1;
                                    }
                                }
                            }
                            let denom = if count_include_pad {
                                kernel[0] * kernel[1]
                            } else {
                                count.max(1)
                            };
                            o_ch[oy * out_w + ox] = sum / denom as f32;
                        }
                        PoolKind::Max => {
                            let mut best = f32::NEG_INFINITY;
                            for ky in 0..kernel[0] {
                                let iy = (oy * strides[0] + ky * dilations[0]) as isize
                                    - pads[0] as isize;
                                if iy < 0 || iy >= h_in as isize {
                                    continue;
                                }
                                let row = &x_ch[(iy as usize) * w_in..(iy as usize + 1) * w_in];
                                for kx in 0..kernel[1] {
                                    let ix = (ox * strides[1] + kx * dilations[1]) as isize
                                        - pads[1] as isize;
                                    if ix >= 0 && ix < w_in as isize {
                                        best = best.max(row[ix as usize]);
                                    }
                                }
                            }
                            o_ch[oy * out_w + ox] = best;
                        }
                    }
                }
            }
        }
    }

    Ok(TensorValue::F32(out, vec![n, c, out_h, out_w]))
}

fn eval_global_average_pool(x_val: &TensorValue) -> Result<TensorValue> {
    let (x, xs) = x_val.as_f32()?;
    anyhow::ensure!(
        xs.len() >= 2,
        "GlobalAveragePool expects rank >= 2, got {xs:?}"
    );
    let n = xs[0];
    let c = xs[1];
    let spatial: usize = xs[2..].iter().product();
    anyhow::ensure!(spatial > 0, "GlobalAveragePool empty spatial dimensions");

    let mut out = Vec::with_capacity(n * c);
    for chunk in x.chunks_exact(spatial) {
        let sum: f32 = chunk.iter().sum();
        out.push(sum / spatial as f32);
    }
    let mut out_shape = vec![n, c];
    out_shape.resize(xs.len(), 1);
    Ok(TensorValue::F32(out, out_shape))
}

fn eval_matmul(a_val: &TensorValue, b_val: &TensorValue) -> Result<TensorValue> {
    let (a, ashape) = a_val.as_f32()?;
    let (b, bshape) = b_val.as_f32()?;

    match (ashape.len(), bshape.len()) {
        (0, _) | (_, 0) => bail!("MatMul does not accept scalar tensors"),
        (1, 1) => {
            anyhow::ensure!(
                ashape[0] == bshape[0],
                "1D MatMul length mismatch: {} vs {}",
                ashape[0],
                bshape[0]
            );
            let dot: f32 = a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum();
            Ok(TensorValue::F32(vec![dot], Vec::new()))
        }
        (1, 2) => {
            let (k, n) = (bshape[0], bshape[1]);
            anyhow::ensure!(
                ashape[0] == k,
                "MatMul shape mismatch: {ashape:?} x {bshape:?}"
            );
            let mut out = vec![0.0f32; n];
            matmul_2d(a, b, &mut out, 1, k, n)?;
            Ok(TensorValue::F32(out, vec![n]))
        }
        (2, 1) => {
            let (m, k) = (ashape[0], ashape[1]);
            anyhow::ensure!(
                bshape[0] == k,
                "MatMul shape mismatch: {ashape:?} x {bshape:?}"
            );
            let mut out = vec![0.0f32; m];
            matmul_2d(a, b, &mut out, m, k, 1)?;
            Ok(TensorValue::F32(out, vec![m]))
        }
        _ => {
            // If one side is 1D and the other is >= 3D, promote via prepend/append 1 and squeeze.
            let prepended = ashape.len() == 1;
            let appended = bshape.len() == 1;
            let a_eff: Vec<usize> = if prepended {
                vec![1, ashape[0]]
            } else {
                ashape.to_vec()
            };
            let b_eff: Vec<usize> = if appended {
                vec![bshape[0], 1]
            } else {
                bshape.to_vec()
            };

            let m = a_eff[a_eff.len() - 2];
            let k_a = a_eff[a_eff.len() - 1];
            let k_b = b_eff[b_eff.len() - 2];
            let n = b_eff[b_eff.len() - 1];
            anyhow::ensure!(
                k_a == k_b,
                "MatMul inner dim mismatch: {ashape:?} x {bshape:?}"
            );

            let a_batch = &a_eff[..a_eff.len() - 2];
            let b_batch = &b_eff[..b_eff.len() - 2];
            let (out_batch, a_bcast, b_bcast) = broadcast_shapes(a_batch, b_batch)?;
            let batch_numel: usize = out_batch.iter().product();
            let a_mat = m * k_a;
            let b_mat = k_a * n;
            let o_mat = m * n;

            let mut out = vec![0.0f32; batch_numel * o_mat];
            let rank = out_batch.len();
            let mut coords = vec![0usize; rank];
            let mut a_b_idx = 0usize;
            let mut b_b_idx = 0usize;

            for batch_i in 0..batch_numel {
                let a_slice = &a[a_b_idx * a_mat..(a_b_idx + 1) * a_mat];
                let b_slice = &b[b_b_idx * b_mat..(b_b_idx + 1) * b_mat];
                let o_slice = &mut out[batch_i * o_mat..(batch_i + 1) * o_mat];
                matmul_2d(a_slice, b_slice, o_slice, m, k_a, n)?;

                for axis in (0..rank).rev() {
                    coords[axis] += 1;
                    a_b_idx += a_bcast[axis];
                    b_b_idx += b_bcast[axis];
                    if coords[axis] < out_batch[axis] {
                        break;
                    }
                    a_b_idx -= coords[axis] * a_bcast[axis];
                    b_b_idx -= coords[axis] * b_bcast[axis];
                    coords[axis] = 0;
                }
            }

            let mut out_shape = out_batch;
            if !prepended {
                out_shape.push(m);
            }
            if !appended {
                out_shape.push(n);
            }
            Ok(TensorValue::F32(out, out_shape))
        }
    }
}

fn transpose_2d(data: &[f32], rows: usize, cols: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            out[c * rows + r] = data[r * cols + c];
        }
    }
    out
}

fn eval_gemm(
    a_val: &TensorValue,
    b_val: &TensorValue,
    c_val: Option<&TensorValue>,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let (a_raw, ashape) = a_val.as_f32()?;
    let (b_raw, bshape) = b_val.as_f32()?;
    anyhow::ensure!(
        ashape.len() == 2 && bshape.len() == 2,
        "Gemm requires 2D A and B, got {ashape:?} and {bshape:?}"
    );
    let trans_a = node.attr_i("transA").unwrap_or(0) != 0;
    let trans_b = node.attr_i("transB").unwrap_or(0) != 0;
    let alpha = node.attr_f("alpha").unwrap_or(1.0);
    let beta = node.attr_f("beta").unwrap_or(1.0);

    let (m, k_a, a_buf) = if trans_a {
        (
            ashape[1],
            ashape[0],
            transpose_2d(a_raw, ashape[0], ashape[1]),
        )
    } else {
        (ashape[0], ashape[1], a_raw.to_vec())
    };
    let (k_b, n, b_buf) = if trans_b {
        (
            bshape[1],
            bshape[0],
            transpose_2d(b_raw, bshape[0], bshape[1]),
        )
    } else {
        (bshape[0], bshape[1], b_raw.to_vec())
    };
    anyhow::ensure!(k_a == k_b, "Gemm inner dimension mismatch: {k_a} vs {k_b}");

    let mut out = vec![0.0f32; m * n];
    matmul_2d(&a_buf, &b_buf, &mut out, m, k_a, n)?;
    if (alpha - 1.0).abs() > f32::EPSILON {
        for val in &mut out {
            *val *= alpha;
        }
    }

    if let Some(c_t) = c_val {
        let (c_data, c_shape) = c_t.as_f32()?;
        let scaled_c: Vec<f32> = if (beta - 1.0).abs() > f32::EPSILON {
            c_data.iter().map(|&v| v * beta).collect()
        } else {
            c_data.to_vec()
        };
        return eval_binary_f32(&out, &[m, n], &scaled_c, c_shape, BinaryOp::Add);
    }

    Ok(TensorValue::F32(out, vec![m, n]))
}

fn eval_softmax(x_val: &TensorValue, node: &ParsedNode) -> Result<TensorValue> {
    let (x, xs) = x_val.as_f32()?;
    anyhow::ensure!(!xs.is_empty(), "Softmax does not support 0D scalar");
    let axis = normalize_axis(node.attr_i("axis").unwrap_or(-1), xs.len())?;
    let outer: usize = xs[..axis].iter().product();
    let axis_dim = xs[axis];
    let inner: usize = xs[axis + 1..].iter().product();
    anyhow::ensure!(axis_dim > 0, "Softmax axis dimension is 0");

    let mut out = vec![0.0f32; x.len()];
    if inner == 1 {
        for (src, dst) in x.chunks_exact(axis_dim).zip(out.chunks_exact_mut(axis_dim)) {
            let max_val = src.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let mut sum = 0.0f32;
            for (d, &s) in dst.iter_mut().zip(src.iter()) {
                let e = (s - max_val).exp();
                *d = e;
                sum += e;
            }
            let inv = 1.0 / sum;
            for d in dst {
                *d *= inv;
            }
        }
    } else {
        for o in 0..outer {
            for i in 0..inner {
                let mut max_val = f32::NEG_INFINITY;
                for a in 0..axis_dim {
                    let idx = (o * axis_dim + a) * inner + i;
                    max_val = max_val.max(x[idx]);
                }
                let mut sum = 0.0f32;
                for a in 0..axis_dim {
                    let idx = (o * axis_dim + a) * inner + i;
                    let e = (x[idx] - max_val).exp();
                    out[idx] = e;
                    sum += e;
                }
                let inv = 1.0 / sum;
                for a in 0..axis_dim {
                    let idx = (o * axis_dim + a) * inner + i;
                    out[idx] *= inv;
                }
            }
        }
    }

    Ok(TensorValue::F32(out, xs.to_vec()))
}

fn eval_reduce_mean(
    x_val: &TensorValue,
    axes_input: Option<&TensorValue>,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let (x, xs) = x_val.as_f32()?;
    let rank = xs.len();
    let keepdims = node.attr_i("keepdims").unwrap_or(1) != 0;
    let noop_empty = node.attr_i("noop_with_empty_axes").unwrap_or(0) != 0;

    let raw_axes: Vec<i64> = if let Some(ax_t) = axes_input {
        let (vals, _) = ax_t.as_i64()?;
        vals.to_vec()
    } else if let Some(vals) = node.attr_ints("axes") {
        vals.to_vec()
    } else {
        Vec::new()
    };

    if raw_axes.is_empty() && noop_empty {
        return Ok(x_val.clone());
    }

    let mut reduce_mask = vec![false; rank];
    if raw_axes.is_empty() {
        reduce_mask.fill(true);
    } else {
        for ax in raw_axes {
            let norm = normalize_axis(ax, rank)?;
            reduce_mask[norm] = true;
        }
    }

    let mut out_shape = Vec::new();
    let mut reduce_shape = Vec::new();
    let mut reduce_count: usize = 1;
    for (axis, &dim) in xs.iter().enumerate() {
        if reduce_mask[axis] {
            reduce_count *= dim;
            reduce_shape.push(1);
            if keepdims {
                out_shape.push(1);
            }
        } else {
            reduce_shape.push(dim);
            out_shape.push(dim);
        }
    }
    anyhow::ensure!(reduce_count > 0, "ReduceMean over zero-sized axis");

    let out_numel: usize = out_shape.iter().product();
    let out_strides = compute_strides(&reduce_shape);
    let mut accum = vec![0.0f64; out_numel];
    let mut coords = vec![0usize; rank];

    for &val in x {
        let mut out_idx = 0usize;
        for axis in 0..rank {
            if !reduce_mask[axis] {
                out_idx += coords[axis] * out_strides[axis];
            }
        }
        accum[out_idx] += f64::from(val);

        for axis in (0..rank).rev() {
            coords[axis] += 1;
            if coords[axis] < xs[axis] {
                break;
            }
            coords[axis] = 0;
        }
    }

    let denom = reduce_count as f64;
    let out: Vec<f32> = accum.into_iter().map(|s| (s / denom) as f32).collect();
    Ok(TensorValue::F32(out, out_shape))
}

fn eval_layer_norm(
    x_val: &TensorValue,
    scale_val: &TensorValue,
    bias_val: Option<&TensorValue>,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let (x, xs) = x_val.as_f32()?;
    let (scale, _) = scale_val.as_f32()?;
    let bias = match bias_val {
        Some(b) => Some(b.as_f32()?.0),
        None => None,
    };
    let axis = normalize_axis(node.attr_i("axis").unwrap_or(-1), xs.len())?;
    let eps = node.attr_f("epsilon").unwrap_or(1e-5);
    let norm_size: usize = xs[axis..].iter().product();
    anyhow::ensure!(
        norm_size > 0 && scale.len() == norm_size,
        "LayerNormalization scale mismatch"
    );
    if let Some(b) = bias {
        anyhow::ensure!(b.len() == norm_size, "LayerNormalization bias mismatch");
    }

    let mut out = vec![0.0f32; x.len()];
    for (src, dst) in x
        .chunks_exact(norm_size)
        .zip(out.chunks_exact_mut(norm_size))
    {
        let mean = src.iter().map(|&v| f64::from(v)).sum::<f64>() / norm_size as f64;
        let var = src
            .iter()
            .map(|&v| {
                let d = f64::from(v) - mean;
                d * d
            })
            .sum::<f64>()
            / norm_size as f64;
        let inv_std = 1.0 / (var + f64::from(eps)).sqrt();
        for (i, d) in dst.iter_mut().enumerate() {
            let norm = ((f64::from(src[i]) - mean) * inv_std) as f32;
            let b_val = bias.map_or(0.0, |b| b[i]);
            *d = norm * scale[i] + b_val;
        }
    }

    Ok(TensorValue::F32(out, xs.to_vec()))
}

fn eval_transpose(x_val: &TensorValue, node: &ParsedNode) -> Result<TensorValue> {
    let xs = x_val.shape();
    let rank = xs.len();
    let perm: Vec<usize> = match node.attr_ints("perm") {
        Some(p) => {
            anyhow::ensure!(
                p.len() == rank,
                "Transpose perm len {} != rank {rank}",
                p.len()
            );
            let mut seen = vec![false; rank];
            let mut out = Vec::with_capacity(rank);
            for &ax in p {
                let u = usize::try_from(ax).context("Negative Transpose perm")?;
                anyhow::ensure!(u < rank && !seen[u], "Invalid Transpose perm {p:?}");
                seen[u] = true;
                out.push(u);
            }
            out
        }
        None => (0..rank).rev().collect(),
    };

    let out_shape: Vec<usize> = perm.iter().map(|&p| xs[p]).collect();
    let in_strides = compute_strides(xs);
    let mut perm_in_strides = vec![0usize; rank];
    for (out_ax, &in_ax) in perm.iter().enumerate() {
        perm_in_strides[out_ax] = in_strides[in_ax];
    }

    let total = x_val.len();
    let mut coords = vec![0usize; rank];
    let mut in_off = 0usize;

    match x_val {
        TensorValue::F32(src, _) => {
            let mut dst = Vec::with_capacity(total);
            for _ in 0..total {
                dst.push(*src.get(in_off).context("Transpose index out of bounds")?);
                for axis in (0..rank).rev() {
                    coords[axis] += 1;
                    in_off += perm_in_strides[axis];
                    if coords[axis] < out_shape[axis] {
                        break;
                    }
                    in_off -= coords[axis] * perm_in_strides[axis];
                    coords[axis] = 0;
                }
            }
            Ok(TensorValue::F32(dst, out_shape))
        }
        TensorValue::I64(src, _) => {
            let mut dst = Vec::with_capacity(total);
            for _ in 0..total {
                dst.push(*src.get(in_off).context("Transpose index out of bounds")?);
                for axis in (0..rank).rev() {
                    coords[axis] += 1;
                    in_off += perm_in_strides[axis];
                    if coords[axis] < out_shape[axis] {
                        break;
                    }
                    in_off -= coords[axis] * perm_in_strides[axis];
                    coords[axis] = 0;
                }
            }
            Ok(TensorValue::I64(dst, out_shape))
        }
    }
}

fn eval_reshape(
    data: &TensorValue,
    shape_val: &TensorValue,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let (raw_shape, _) = shape_val.as_i64()?;
    let allowzero = node.attr_i("allowzero").unwrap_or(0) != 0;
    let in_shape = data.shape();
    let total = data.len();

    let mut out_dims: Vec<Option<usize>> = Vec::with_capacity(raw_shape.len());
    let mut infer_idx: Option<usize> = None;
    let mut known_prod: usize = 1;

    for (idx, &d) in raw_shape.iter().enumerate() {
        if d == -1 {
            anyhow::ensure!(infer_idx.is_none(), "Reshape allows at most one -1 dim");
            infer_idx = Some(idx);
            out_dims.push(None);
        } else {
            let dim = if d == 0 && !allowzero {
                *in_shape
                    .get(idx)
                    .with_context(|| format!("Reshape 0 dim at index {idx} exceeds rank"))?
            } else {
                usize::try_from(d).with_context(|| format!("Invalid Reshape dim {d}"))?
            };
            known_prod = known_prod
                .checked_mul(dim)
                .context("Reshape dimension overflow")?;
            out_dims.push(Some(dim));
        }
    }

    let final_shape: Vec<usize> = if let Some(neg_idx) = infer_idx {
        anyhow::ensure!(
            known_prod > 0 && total.is_multiple_of(known_prod),
            "Cannot reshape {total} elements into {raw_shape:?}"
        );
        let inferred = total / known_prod;
        out_dims[neg_idx] = Some(inferred);
        out_dims.into_iter().flatten().collect()
    } else {
        anyhow::ensure!(
            known_prod == total,
            "Reshape element count mismatch: {known_prod} != {total}"
        );
        out_dims.into_iter().flatten().collect()
    };

    Ok(data.with_shape(final_shape))
}

fn eval_squeeze(
    data: &TensorValue,
    axes_input: Option<&TensorValue>,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let in_shape = data.shape();
    let rank = in_shape.len();
    let raw_axes: Option<Vec<i64>> = if let Some(ax_t) = axes_input {
        Some(ax_t.as_i64()?.0.to_vec())
    } else {
        node.attr_ints("axes").map(<[i64]>::to_vec)
    };

    let out_shape: Vec<usize> = if let Some(axes) = raw_axes {
        let mut drop = vec![false; rank];
        for ax in axes {
            let norm = normalize_axis(ax, rank)?;
            anyhow::ensure!(
                in_shape[norm] == 1,
                "Cannot squeeze axis {norm} of size {}",
                in_shape[norm]
            );
            drop[norm] = true;
        }
        in_shape
            .iter()
            .enumerate()
            .filter_map(|(i, &d)| (!drop[i]).then_some(d))
            .collect()
    } else {
        in_shape.iter().copied().filter(|&d| d != 1).collect()
    };

    Ok(data.with_shape(out_shape))
}

fn eval_unsqueeze(
    data: &TensorValue,
    axes_input: Option<&TensorValue>,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let raw_axes: Vec<i64> = if let Some(ax_t) = axes_input {
        ax_t.as_i64()?.0.to_vec()
    } else if let Some(vals) = node.attr_ints("axes") {
        vals.to_vec()
    } else {
        bail!("Unsqueeze requires axes input or attribute");
    };

    let in_shape = data.shape();
    let out_rank = in_shape.len() + raw_axes.len();
    let mut insert_at = vec![false; out_rank];
    for ax in raw_axes {
        let norm = normalize_axis(ax, out_rank)?;
        anyhow::ensure!(!insert_at[norm], "Duplicate Unsqueeze axis {norm}");
        insert_at[norm] = true;
    }

    let mut out_shape = Vec::with_capacity(out_rank);
    let mut in_iter = in_shape.iter().copied();
    for is_new in insert_at {
        if is_new {
            out_shape.push(1);
        } else if let Some(d) = in_iter.next() {
            out_shape.push(d);
        }
    }

    Ok(data.with_shape(out_shape))
}

fn eval_slice(
    data: &TensorValue,
    starts_t: Option<&TensorValue>,
    ends_t: Option<&TensorValue>,
    axes_t: Option<&TensorValue>,
    steps_t: Option<&TensorValue>,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let in_shape = data.shape();
    let rank = in_shape.len();

    let starts: Vec<i64> = if let Some(t) = starts_t {
        t.as_i64()?.0.to_vec()
    } else if let Some(a) = node.attr_ints("starts") {
        a.to_vec()
    } else {
        bail!("Slice requires starts");
    };
    let ends: Vec<i64> = if let Some(t) = ends_t {
        t.as_i64()?.0.to_vec()
    } else if let Some(a) = node.attr_ints("ends") {
        a.to_vec()
    } else {
        bail!("Slice requires ends");
    };
    anyhow::ensure!(
        starts.len() == ends.len(),
        "Slice starts/ends length mismatch"
    );

    let axes: Vec<i64> = if let Some(t) = axes_t {
        t.as_i64()?.0.to_vec()
    } else if let Some(a) = node.attr_ints("axes") {
        a.to_vec()
    } else {
        (0..starts.len() as i64).collect()
    };
    let steps: Vec<i64> = if let Some(t) = steps_t {
        t.as_i64()?.0.to_vec()
    } else {
        vec![1i64; starts.len()]
    };
    anyhow::ensure!(
        axes.len() == starts.len() && steps.len() == starts.len(),
        "Slice axes/steps length mismatch"
    );

    let mut axis_start: Vec<i64> = vec![0; rank];
    let mut axis_step: Vec<i64> = vec![1; rank];
    let mut out_shape: Vec<usize> = in_shape.to_vec();

    for i in 0..starts.len() {
        let ax = normalize_axis(axes[i], rank)?;
        let dim = i64::try_from(in_shape[ax]).context("Slice dim overflow")?;
        let step = steps[i];
        anyhow::ensure!(step != 0 && step != i64::MIN, "Invalid Slice step {step}");

        let raw_s = starts[i];
        let raw_e = ends[i];
        if step > 0 {
            let s = (if raw_s < 0 { raw_s + dim } else { raw_s }).clamp(0, dim);
            let e = (if raw_e < 0 { raw_e + dim } else { raw_e }).clamp(0, dim);
            let len = if e > s {
                usize::try_from((e - s + step - 1) / step).unwrap_or(0)
            } else {
                0
            };
            axis_start[ax] = s;
            axis_step[ax] = step;
            out_shape[ax] = len;
        } else {
            let s = (if raw_s < 0 { raw_s + dim } else { raw_s }).clamp(-1, dim - 1);
            let e = (if raw_e < 0 { raw_e + dim } else { raw_e }).clamp(-1, dim - 1);
            let abs_step = -step;
            let len = if s > e {
                usize::try_from((s - e + abs_step - 1) / abs_step).unwrap_or(0)
            } else {
                0
            };
            axis_start[ax] = s;
            axis_step[ax] = step;
            out_shape[ax] = len;
        }
    }

    let out_numel: usize = out_shape.iter().product();
    if out_numel == 0 {
        return Ok(match data {
            TensorValue::F32(_, _) => TensorValue::F32(Vec::new(), out_shape),
            TensorValue::I64(_, _) => TensorValue::I64(Vec::new(), out_shape),
        });
    }

    let in_strides = compute_strides(in_shape);
    let mut coords = vec![0usize; rank];
    let in_idx_at = |coords: &[usize]| -> usize {
        let mut off = 0isize;
        for ax in 0..rank {
            let src_c = axis_start[ax] + (coords[ax] as i64) * axis_step[ax];
            off += (src_c as isize) * (in_strides[ax] as isize);
        }
        off as usize
    };

    match data {
        TensorValue::F32(src, _) => {
            let mut dst = Vec::with_capacity(out_numel);
            for _ in 0..out_numel {
                let idx = in_idx_at(&coords);
                dst.push(*src.get(idx).context("Slice source index out of bounds")?);
                for ax in (0..rank).rev() {
                    coords[ax] += 1;
                    if coords[ax] < out_shape[ax] {
                        break;
                    }
                    coords[ax] = 0;
                }
            }
            Ok(TensorValue::F32(dst, out_shape))
        }
        TensorValue::I64(src, _) => {
            let mut dst = Vec::with_capacity(out_numel);
            for _ in 0..out_numel {
                let idx = in_idx_at(&coords);
                dst.push(*src.get(idx).context("Slice source index out of bounds")?);
                for ax in (0..rank).rev() {
                    coords[ax] += 1;
                    if coords[ax] < out_shape[ax] {
                        break;
                    }
                    coords[ax] = 0;
                }
            }
            Ok(TensorValue::I64(dst, out_shape))
        }
    }
}

fn eval_concat(inputs: &[&TensorValue], node: &ParsedNode) -> Result<TensorValue> {
    anyhow::ensure!(!inputs.is_empty(), "Concat requires at least 1 input");
    let first_shape = inputs[0].shape();
    let rank = first_shape.len();
    anyhow::ensure!(rank > 0, "Concat cannot operate on 0D scalars");
    let axis = normalize_axis(node.attr_i("axis").context("Concat missing axis")?, rank)?;

    let mut out_shape = first_shape.to_vec();
    out_shape[axis] = 0;
    for inp in inputs {
        let s = inp.shape();
        anyhow::ensure!(s.len() == rank, "Concat rank mismatch");
        for (ax, (&d, &fd)) in s.iter().zip(first_shape.iter()).enumerate() {
            if ax == axis {
                out_shape[axis] += d;
            } else {
                anyhow::ensure!(d == fd, "Concat shape mismatch at axis {ax}: {d} vs {fd}");
            }
        }
    }

    let outer: usize = first_shape[..axis].iter().product();
    let inner: usize = first_shape[axis + 1..].iter().product();

    match inputs[0] {
        TensorValue::F32(_, _) => {
            let mut out = Vec::with_capacity(out_shape.iter().product());
            let slices: Vec<(&[f32], usize)> = inputs
                .iter()
                .map(|t| {
                    let (d, s) = t.as_f32()?;
                    Ok((d, s[axis] * inner))
                })
                .collect::<Result<_>>()?;
            for o in 0..outer {
                for &(data, chunk) in &slices {
                    out.extend_from_slice(&data[o * chunk..(o + 1) * chunk]);
                }
            }
            Ok(TensorValue::F32(out, out_shape))
        }
        TensorValue::I64(_, _) => {
            let mut out = Vec::with_capacity(out_shape.iter().product());
            let slices: Vec<(&[i64], usize)> = inputs
                .iter()
                .map(|t| {
                    let (d, s) = t.as_i64()?;
                    Ok((d, s[axis] * inner))
                })
                .collect::<Result<_>>()?;
            for o in 0..outer {
                for &(data, chunk) in &slices {
                    out.extend_from_slice(&data[o * chunk..(o + 1) * chunk]);
                }
            }
            Ok(TensorValue::I64(out, out_shape))
        }
    }
}

fn eval_gather(
    data: &TensorValue,
    indices_val: &TensorValue,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let ds = data.shape();
    anyhow::ensure!(!ds.is_empty(), "Gather cannot index into 0D scalar");
    let axis = normalize_axis(node.attr_i("axis").unwrap_or(0), ds.len())?;
    let (indices, idx_shape) = indices_val.as_i64()?;

    let mut out_shape = Vec::with_capacity(ds.len() - 1 + idx_shape.len());
    out_shape.extend_from_slice(&ds[..axis]);
    out_shape.extend_from_slice(idx_shape);
    out_shape.extend_from_slice(&ds[axis + 1..]);

    let outer: usize = ds[..axis].iter().product();
    let axis_dim = ds[axis];
    let axis_dim_i = i64::try_from(axis_dim).context("Gather axis dim overflow")?;
    let inner: usize = ds[axis + 1..].iter().product();

    let mut norm_indices = Vec::with_capacity(indices.len());
    for &raw in indices {
        let idx = if raw < 0 { raw + axis_dim_i } else { raw };
        anyhow::ensure!(
            idx >= 0 && idx < axis_dim_i,
            "Gather index {raw} out of bounds for axis dim {axis_dim}"
        );
        norm_indices.push(idx as usize);
    }

    match data {
        TensorValue::F32(src, _) => {
            let mut dst = Vec::with_capacity(out_shape.iter().product());
            for o in 0..outer {
                for &idx in &norm_indices {
                    let start = (o * axis_dim + idx) * inner;
                    dst.extend_from_slice(&src[start..start + inner]);
                }
            }
            Ok(TensorValue::F32(dst, out_shape))
        }
        TensorValue::I64(src, _) => {
            let mut dst = Vec::with_capacity(out_shape.iter().product());
            for o in 0..outer {
                for &idx in &norm_indices {
                    let start = (o * axis_dim + idx) * inner;
                    dst.extend_from_slice(&src[start..start + inner]);
                }
            }
            Ok(TensorValue::I64(dst, out_shape))
        }
    }
}

fn eval_split(
    data: &TensorValue,
    split_input: Option<&TensorValue>,
    node: &ParsedNode,
) -> Result<Vec<TensorValue>> {
    let ds = data.shape();
    anyhow::ensure!(!ds.is_empty(), "Split cannot operate on 0D scalar");
    let axis = normalize_axis(node.attr_i("axis").unwrap_or(0), ds.len())?;
    let dim_len = ds[axis];
    let num_outputs = node
        .attr_i("num_outputs")
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(node.outputs.len())
        .max(1);

    let splits: Vec<usize> = if let Some(sp_t) = split_input {
        sp_t.as_i64()?
            .0
            .iter()
            .map(|&v| usize::try_from(v).context("Negative Split size"))
            .collect::<Result<_>>()?
    } else if let Some(sp_attr) = node.attr_ints("split") {
        sp_attr
            .iter()
            .map(|&v| usize::try_from(v).context("Negative Split size"))
            .collect::<Result<_>>()?
    } else {
        let base = dim_len / num_outputs;
        let rem = dim_len % num_outputs;
        (0..num_outputs)
            .map(|i| base + usize::from(i < rem))
            .collect()
    };

    let sum_splits: usize = splits.iter().sum();
    anyhow::ensure!(
        sum_splits == dim_len,
        "Split sizes {splits:?} sum {sum_splits} != axis length {dim_len}"
    );

    let outer: usize = ds[..axis].iter().product();
    let inner: usize = ds[axis + 1..].iter().product();

    let mut outputs = Vec::with_capacity(splits.len());
    let mut axis_offset = 0usize;
    for split_len in splits {
        let mut out_shape = ds.to_vec();
        out_shape[axis] = split_len;
        let chunk_elems = split_len * inner;
        let out_val = match data {
            TensorValue::F32(src, _) => {
                let mut buf = Vec::with_capacity(outer * chunk_elems);
                for o in 0..outer {
                    let start = (o * dim_len + axis_offset) * inner;
                    buf.extend_from_slice(&src[start..start + chunk_elems]);
                }
                TensorValue::F32(buf, out_shape)
            }
            TensorValue::I64(src, _) => {
                let mut buf = Vec::with_capacity(outer * chunk_elems);
                for o in 0..outer {
                    let start = (o * dim_len + axis_offset) * inner;
                    buf.extend_from_slice(&src[start..start + chunk_elems]);
                }
                TensorValue::I64(buf, out_shape)
            }
        };
        outputs.push(out_val);
        axis_offset += split_len;
    }

    Ok(outputs)
}

fn eval_expand(data: &TensorValue, shape_t: &TensorValue) -> Result<TensorValue> {
    let (target_raw, _) = shape_t.as_i64()?;
    let target_shape: Vec<usize> = target_raw
        .iter()
        .map(|&d| usize::try_from(d).context("Negative Expand dim"))
        .collect::<Result<_>>()?;
    let (out_shape, a_strides, _) = broadcast_shapes(data.shape(), &target_shape)?;
    let total: usize = out_shape.iter().product();
    let rank = out_shape.len();
    let mut coords = vec![0usize; rank];
    let mut off = 0usize;

    match data {
        TensorValue::F32(src, _) => {
            let mut dst = Vec::with_capacity(total);
            for _ in 0..total {
                dst.push(*src.get(off).context("Expand index out of bounds")?);
                for ax in (0..rank).rev() {
                    coords[ax] += 1;
                    off += a_strides[ax];
                    if coords[ax] < out_shape[ax] {
                        break;
                    }
                    off -= coords[ax] * a_strides[ax];
                    coords[ax] = 0;
                }
            }
            Ok(TensorValue::F32(dst, out_shape))
        }
        TensorValue::I64(src, _) => {
            let mut dst = Vec::with_capacity(total);
            for _ in 0..total {
                dst.push(*src.get(off).context("Expand index out of bounds")?);
                for ax in (0..rank).rev() {
                    coords[ax] += 1;
                    off += a_strides[ax];
                    if coords[ax] < out_shape[ax] {
                        break;
                    }
                    off -= coords[ax] * a_strides[ax];
                    coords[ax] = 0;
                }
            }
            Ok(TensorValue::I64(dst, out_shape))
        }
    }
}

fn eval_pad(
    data: &TensorValue,
    pads_t: Option<&TensorValue>,
    const_t: Option<&TensorValue>,
    node: &ParsedNode,
) -> Result<TensorValue> {
    let in_shape = data.shape();
    let rank = in_shape.len();
    let raw_pads: Vec<i64> = if let Some(t) = pads_t {
        t.as_i64()?.0.to_vec()
    } else if let Some(a) = node.attr_ints("pads") {
        a.to_vec()
    } else {
        bail!("Pad missing pads");
    };
    anyhow::ensure!(raw_pads.len() == rank * 2, "Pad length mismatch");
    let mode = node.attr_s("mode").unwrap_or("constant");
    anyhow::ensure!(
        mode == "constant" || mode == "edge",
        "Unsupported Pad mode '{mode}'"
    );

    let mut out_shape = Vec::with_capacity(rank);
    let mut pad_before = Vec::with_capacity(rank);
    for ax in 0..rank {
        let pb = usize::try_from(raw_pads[ax]).context("Negative pad_before")?;
        let pa = usize::try_from(raw_pads[ax + rank]).context("Negative pad_after")?;
        pad_before.push(pb);
        out_shape.push(in_shape[ax] + pb + pa);
    }

    let total: usize = out_shape.iter().product();
    let in_strides = compute_strides(in_shape);
    let mut coords = vec![0usize; rank];

    match data {
        TensorValue::F32(src, _) => {
            let fill = const_t
                .and_then(|t| t.as_f32().ok())
                .and_then(|(v, _)| v.first().copied())
                .or_else(|| node.attr_f("value"))
                .unwrap_or(0.0);
            let mut dst = Vec::with_capacity(total);
            for _ in 0..total {
                let mut in_bounds = true;
                let mut idx = 0usize;
                for ax in 0..rank {
                    let c = coords[ax] as isize - pad_before[ax] as isize;
                    if c < 0 || c >= in_shape[ax] as isize {
                        if mode == "edge" && in_shape[ax] > 0 {
                            let clamped = c.clamp(0, in_shape[ax] as isize - 1) as usize;
                            idx += clamped * in_strides[ax];
                        } else {
                            in_bounds = false;
                            break;
                        }
                    } else {
                        idx += (c as usize) * in_strides[ax];
                    }
                }
                dst.push(if in_bounds { src[idx] } else { fill });
                for ax in (0..rank).rev() {
                    coords[ax] += 1;
                    if coords[ax] < out_shape[ax] {
                        break;
                    }
                    coords[ax] = 0;
                }
            }
            Ok(TensorValue::F32(dst, out_shape))
        }
        TensorValue::I64(src, _) => {
            let fill = const_t
                .and_then(|t| t.as_i64().ok())
                .and_then(|(v, _)| v.first().copied())
                .unwrap_or(0);
            let mut dst = Vec::with_capacity(total);
            for _ in 0..total {
                let mut in_bounds = true;
                let mut idx = 0usize;
                for ax in 0..rank {
                    let c = coords[ax] as isize - pad_before[ax] as isize;
                    if c < 0 || c >= in_shape[ax] as isize {
                        if mode == "edge" && in_shape[ax] > 0 {
                            let clamped = c.clamp(0, in_shape[ax] as isize - 1) as usize;
                            idx += clamped * in_strides[ax];
                        } else {
                            in_bounds = false;
                            break;
                        }
                    } else {
                        idx += (c as usize) * in_strides[ax];
                    }
                }
                dst.push(if in_bounds { src[idx] } else { fill });
                for ax in (0..rank).rev() {
                    coords[ax] += 1;
                    if coords[ax] < out_shape[ax] {
                        break;
                    }
                    coords[ax] = 0;
                }
            }
            Ok(TensorValue::I64(dst, out_shape))
        }
    }
}

fn erf_approx(x: f32) -> f32 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let ax = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * ax);
    let y = 1.0
        - (((((1.061_405_4 * t - 1.453_152) * t) + 1.421_413_7) * t - 0.284_496_74) * t
            + 0.254_829_6)
            * t
            * (-ax * ax).exp();
    sign * y
}

fn eval_node(
    node: &ParsedNode,
    values: &HashMap<String, TensorValue>,
    initializers: &HashMap<String, TensorValue>,
) -> Result<Vec<TensorValue>> {
    let get_in = |idx: usize| -> Result<&TensorValue> {
        let name = node
            .inputs
            .get(idx)
            .filter(|n| !n.is_empty())
            .with_context(|| format!("Node '{}' missing required input {idx}", node.op_type))?;
        values
            .get(name)
            .or_else(|| initializers.get(name))
            .with_context(|| format!("Missing tensor '{name}' for node '{}'", node.op_type))
    };
    let get_opt = |idx: usize| -> Result<Option<&TensorValue>> {
        let Some(name) = node.inputs.get(idx).filter(|n| !n.is_empty()) else {
            return Ok(None);
        };
        let val = values
            .get(name)
            .or_else(|| initializers.get(name))
            .with_context(|| format!("Missing optional tensor '{name}' for '{}'", node.op_type))?;
        Ok(Some(val))
    };

    let single = match node.op_type.as_str() {
        "Constant" => {
            if let Some(t) = node.attr("value").and_then(|a| a.t.as_ref()) {
                t.clone()
            } else if let Some(f) = node.attr_f("value_float") {
                TensorValue::F32(vec![f], Vec::new())
            } else if let Some(fs) = node.attr_floats("value_floats") {
                TensorValue::F32(fs.to_vec(), vec![fs.len()])
            } else if let Some(i) = node.attr_i("value_int") {
                TensorValue::I64(vec![i], Vec::new())
            } else if let Some(is) = node.attr_ints("value_ints") {
                TensorValue::I64(is.to_vec(), vec![is.len()])
            } else {
                bail!("Constant node missing value attribute");
            }
        }
        "Identity" | "Dropout" => get_in(0)?.clone(),
        "Shape" => {
            let s = get_in(0)?.shape();
            let rank_i = s.len() as i64;
            let start = node
                .attr_i("start")
                .map(|v| (if v < 0 { v + rank_i } else { v }).clamp(0, rank_i) as usize)
                .unwrap_or(0);
            let end = node
                .attr_i("end")
                .map(|v| (if v < 0 { v + rank_i } else { v }).clamp(0, rank_i) as usize)
                .unwrap_or(s.len())
                .max(start);
            let dims: Vec<i64> = s[start..end].iter().map(|&d| d as i64).collect();
            let len = dims.len();
            TensorValue::I64(dims, vec![len])
        }
        "Cast" => {
            let x = get_in(0)?;
            let to = node.attr_i("to").context("Cast missing 'to' attribute")?;
            match to {
                1 | 10 | 11 => TensorValue::F32(x.to_f32_vec(), x.shape().to_vec()),
                6 | 7 => match x {
                    TensorValue::F32(v, s) => {
                        TensorValue::I64(v.iter().map(|&f| f as i64).collect(), s.clone())
                    }
                    TensorValue::I64(_, _) => x.clone(),
                },
                9 => match x {
                    TensorValue::F32(v, s) => TensorValue::I64(
                        v.iter().map(|&f| i64::from(f != 0.0)).collect(),
                        s.clone(),
                    ),
                    TensorValue::I64(v, s) => {
                        TensorValue::I64(v.iter().map(|&i| i64::from(i != 0)).collect(), s.clone())
                    }
                },
                other => bail!("Unsupported Cast target type {other}"),
            }
        }
        "Conv" => eval_conv(get_in(0)?, get_in(1)?, get_opt(2)?, node)?,
        "BatchNormalization" => eval_batch_norm(
            get_in(0)?,
            get_in(1)?,
            get_in(2)?,
            get_in(3)?,
            get_in(4)?,
            node,
        )?,
        "AveragePool" => eval_pool2d(get_in(0)?, node, PoolKind::Average)?,
        "MaxPool" => eval_pool2d(get_in(0)?, node, PoolKind::Max)?,
        "GlobalAveragePool" => eval_global_average_pool(get_in(0)?)?,
        "MatMul" => eval_matmul(get_in(0)?, get_in(1)?)?,
        "Gemm" => eval_gemm(get_in(0)?, get_in(1)?, get_opt(2)?, node)?,
        "Add" => eval_binary(get_in(0)?, get_in(1)?, BinaryOp::Add)?,
        "Sub" => eval_binary(get_in(0)?, get_in(1)?, BinaryOp::Sub)?,
        "Mul" => eval_binary(get_in(0)?, get_in(1)?, BinaryOp::Mul)?,
        "Div" => eval_binary(get_in(0)?, get_in(1)?, BinaryOp::Div)?,
        "Pow" => eval_binary(get_in(0)?, get_in(1)?, BinaryOp::Pow)?,
        "Sqrt" => map_f32(get_in(0)?, f32::sqrt)?,
        "Relu" => map_f32(get_in(0)?, |x| x.max(0.0))?,
        "Sigmoid" => map_f32(get_in(0)?, |x| 1.0 / (1.0 + (-x).exp()))?,
        "Tanh" => map_f32(get_in(0)?, f32::tanh)?,
        "Exp" => map_f32(get_in(0)?, f32::exp)?,
        "Neg" => match get_in(0)? {
            TensorValue::F32(v, s) => TensorValue::F32(v.iter().map(|&x| -x).collect(), s.clone()),
            TensorValue::I64(v, s) => {
                TensorValue::I64(v.iter().map(|&x| x.wrapping_neg()).collect(), s.clone())
            }
        },
        "Abs" => match get_in(0)? {
            TensorValue::F32(v, s) => {
                TensorValue::F32(v.iter().map(|&x| x.abs()).collect(), s.clone())
            }
            TensorValue::I64(v, s) => {
                TensorValue::I64(v.iter().map(|&x| x.wrapping_abs()).collect(), s.clone())
            }
        },
        "Floor" => map_f32(get_in(0)?, f32::floor)?,
        "Ceil" => map_f32(get_in(0)?, f32::ceil)?,
        "Erf" => map_f32(get_in(0)?, erf_approx)?,
        "Swish" | "Silu" => map_f32(get_in(0)?, |x| x / (1.0 + (-x).exp()))?,
        "Clip" => {
            let x = get_in(0)?;
            let min_v = get_opt(1)?
                .and_then(|t| t.to_f32_vec().first().copied())
                .or_else(|| node.attr_f("min"))
                .unwrap_or(f32::NEG_INFINITY);
            let max_v = get_opt(2)?
                .and_then(|t| t.to_f32_vec().first().copied())
                .or_else(|| node.attr_f("max"))
                .unwrap_or(f32::INFINITY);
            map_f32(x, |v| v.clamp(min_v, max_v))?
        }
        "HardSigmoid" => {
            let alpha = node.attr_f("alpha").unwrap_or(0.2);
            let beta = node.attr_f("beta").unwrap_or(0.5);
            map_f32(get_in(0)?, |x| (alpha * x + beta).clamp(0.0, 1.0))?
        }
        "HardSwish" => map_f32(get_in(0)?, |x| x * ((x / 6.0 + 0.5).clamp(0.0, 1.0)))?,
        "Softmax" => eval_softmax(get_in(0)?, node)?,
        "ReduceMean" => eval_reduce_mean(get_in(0)?, get_opt(1)?, node)?,
        "LayerNormalization" => eval_layer_norm(get_in(0)?, get_in(1)?, get_opt(2)?, node)?,
        "Transpose" => eval_transpose(get_in(0)?, node)?,
        "Reshape" => eval_reshape(get_in(0)?, get_in(1)?, node)?,
        "Flatten" => {
            let x = get_in(0)?;
            let xs = x.shape();
            let axis = normalize_axis(node.attr_i("axis").unwrap_or(1), xs.len() + 1)?;
            let d0: usize = xs[..axis].iter().product();
            let d1: usize = xs[axis..].iter().product();
            x.with_shape(vec![d0, d1])
        }
        "Squeeze" => eval_squeeze(get_in(0)?, get_opt(1)?, node)?,
        "Unsqueeze" => eval_unsqueeze(get_in(0)?, get_opt(1)?, node)?,
        "Slice" => eval_slice(
            get_in(0)?,
            get_opt(1)?,
            get_opt(2)?,
            get_opt(3)?,
            get_opt(4)?,
            node,
        )?,
        "Concat" => {
            let mut inputs = Vec::with_capacity(node.inputs.len());
            for idx in 0..node.inputs.len() {
                inputs.push(get_in(idx)?);
            }
            eval_concat(&inputs, node)?
        }
        "Gather" => eval_gather(get_in(0)?, get_in(1)?, node)?,
        "Split" => return eval_split(get_in(0)?, get_opt(1)?, node),
        "Expand" => eval_expand(get_in(0)?, get_in(1)?)?,
        "Pad" => eval_pad(get_in(0)?, get_opt(1)?, get_opt(2)?, node)?,
        "ConstantOfShape" => {
            let (raw_shape, _) = get_in(0)?.as_i64()?;
            let shape: Vec<usize> = raw_shape
                .iter()
                .map(|&d| usize::try_from(d).context("Negative ConstantOfShape dim"))
                .collect::<Result<_>>()?;
            let numel: usize = shape.iter().product();
            match node.attr("value").and_then(|a| a.t.as_ref()) {
                Some(TensorValue::I64(v, _)) => {
                    let fill = v.first().copied().unwrap_or(0);
                    TensorValue::I64(vec![fill; numel], shape)
                }
                Some(TensorValue::F32(v, _)) => {
                    let fill = v.first().copied().unwrap_or(0.0);
                    TensorValue::F32(vec![fill; numel], shape)
                }
                None => TensorValue::F32(vec![0.0; numel], shape),
            }
        }
        other => bail!("Unsupported ONNX operator '{other}'"),
    };

    Ok(vec![single])
}

fn run_graph(
    model: &ParsedModel,
    input: TensorValue,
    cancel: &CancellationToken,
) -> Result<TensorValue> {
    let mut values: HashMap<String, TensorValue> = HashMap::with_capacity(model.nodes.len() + 1);
    values.insert(model.input_name.clone(), input);

    for (idx, node) in model.nodes.iter().enumerate() {
        if idx.is_multiple_of(16) && cancel.is_cancelled() {
            bail!("OCR cancelled by user");
        }
        let outputs = eval_node(node, &values, &model.initializers)?;
        for (out_name, out_val) in node.outputs.iter().zip(outputs) {
            if !out_name.is_empty() {
                values.insert(out_name.clone(), out_val);
            }
        }
        for inp_name in &node.inputs {
            if !inp_name.is_empty() && model.last_use.get(inp_name) == Some(&idx) {
                values.remove(inp_name);
            }
        }
    }

    values
        .remove(&model.output_name)
        .or_else(|| model.initializers.get(&model.output_name).cloned())
        .with_context(|| format!("Graph did not produce output '{}'", model.output_name))
}

// ============================================================================
// Subtitle Preprocessing, Line Segmentation & CTC Decoding
// ============================================================================

fn median_u8(mut vals: Vec<u8>) -> u8 {
    if vals.is_empty() {
        return 0;
    }
    let mid = vals.len() / 2;
    let (_, median, _) = vals.select_nth_unstable(mid);
    *median
}

fn estimate_background_rgb(rgba: &[u8], width: usize, height: usize) -> [u8; 3] {
    let mut rs = Vec::with_capacity(width * 2 + height * 2);
    let mut gs = Vec::with_capacity(width * 2 + height * 2);
    let mut bs = Vec::with_capacity(width * 2 + height * 2);

    let mut push_px = |x: usize, y: usize| {
        let off = (y * width + x) * 4;
        if let Some(px) = rgba.get(off..off + 4) {
            rs.push(px[0]);
            gs.push(px[1]);
            bs.push(px[2]);
        }
    };

    for x in 0..width {
        push_px(x, 0);
        if height > 1 {
            push_px(x, height - 1);
        }
    }
    for y in 1..height.saturating_sub(1) {
        push_px(0, y);
        if width > 1 {
            push_px(width - 1, y);
        }
    }

    [median_u8(rs), median_u8(gs), median_u8(bs)]
}

fn pixel_luma(rgb: [u8; 3]) -> f32 {
    0.299 * f32::from(rgb[0]) + 0.587 * f32::from(rgb[1]) + 0.114 * f32::from(rgb[2])
}

fn is_foreground_pixel(px: [u8; 4], bg: [u8; 3], bg_luma: f32) -> bool {
    if px[3] < 32 {
        return false;
    }
    let dr = (i32::from(px[0]) - i32::from(bg[0])).abs();
    let dg = (i32::from(px[1]) - i32::from(bg[1])).abs();
    let db = (i32::from(px[2]) - i32::from(bg[2])).abs();
    let max_diff = dr.max(dg).max(db);
    let luma_diff = (pixel_luma([px[0], px[1], px[2]]) - bg_luma).abs();
    max_diff > 28 || luma_diff > 24.0
}

/// Segments a rendered cue RGBA image into `(x0, y0, x1, y1)` inclusive line bounding boxes.
fn segment_text_lines(
    rgba: &[u8],
    width: usize,
    height: usize,
    bg: [u8; 3],
) -> Vec<(usize, usize, usize, usize)> {
    let bg_luma = pixel_luma(bg);
    let mut row_bounds: Vec<Option<(usize, usize)>> = Vec::with_capacity(height);
    let mut global_min_x = usize::MAX;
    let mut global_max_x = 0usize;
    let mut global_min_y = usize::MAX;
    let mut global_max_y = 0usize;
    let mut has_fg = false;

    for y in 0..height {
        let mut r_min = usize::MAX;
        let mut r_max = 0usize;
        let mut row_fg = false;
        let row_off = y * width * 4;
        for x in 0..width {
            let off = row_off + x * 4;
            let Some(s) = rgba.get(off..off + 4) else {
                continue;
            };
            if is_foreground_pixel([s[0], s[1], s[2], s[3]], bg, bg_luma) {
                r_min = r_min.min(x);
                r_max = r_max.max(x);
                row_fg = true;
                has_fg = true;
                global_min_x = global_min_x.min(x);
                global_max_x = global_max_x.max(x);
                global_min_y = global_min_y.min(y);
                global_max_y = global_max_y.max(y);
            }
        }
        row_bounds.push(row_fg.then_some((r_min, r_max)));
    }

    if !has_fg {
        return Vec::new();
    }

    let mut raw_bands: Vec<(usize, usize)> = Vec::new();
    let mut in_band_start: Option<usize> = None;
    for (y, bounds) in row_bounds.iter().enumerate() {
        match (bounds.is_some(), in_band_start) {
            (true, None) => in_band_start = Some(y),
            (false, Some(start)) => {
                raw_bands.push((start, y - 1));
                in_band_start = None;
            }
            _ => {}
        }
    }
    if let Some(start) = in_band_start {
        raw_bands.push((start, height - 1));
    }

    // Merge vertical gaps <= MAX_VERTICAL_GAP_PX (3px) between active bands.
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in raw_bands {
        if let Some(prev) = merged.last_mut() {
            let gap = start.saturating_sub(prev.1 + 1);
            if gap <= MAX_VERTICAL_GAP_PX {
                prev.1 = end;
                continue;
            }
        }
        merged.push((start, end));
    }

    let mut out_boxes = Vec::new();
    for (y0, y1) in merged {
        if (y1 - y0 + 1) < MIN_LINE_HEIGHT_PX {
            continue;
        }
        let mut x0 = usize::MAX;
        let mut x1 = 0usize;
        for bounds in row_bounds[y0..=y1].iter().flatten() {
            x0 = x0.min(bounds.0);
            x1 = x1.max(bounds.1);
        }
        if x0 <= x1 {
            out_boxes.push(pad_box(x0, y0, x1, y1, width, height));
        }
    }

    if out_boxes.is_empty() {
        out_boxes.push(pad_box(
            global_min_x,
            global_min_y,
            global_max_x,
            global_max_y,
            width,
            height,
        ));
    }

    out_boxes
}

fn pad_box(
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    width: usize,
    height: usize,
) -> (usize, usize, usize, usize) {
    (
        x0.saturating_sub(LINE_PAD_PX),
        y0.saturating_sub(LINE_PAD_PX),
        (x1 + LINE_PAD_PX).min(width - 1),
        (y1 + LINE_PAD_PX).min(height - 1),
    )
}

fn sample_rgb_over_bg(rgba: &[u8], img_w: usize, x: usize, y: usize, bg: [u8; 3]) -> [f32; 3] {
    let off = (y * img_w + x) * 4;
    let Some(px) = rgba.get(off..off + 4) else {
        return [f32::from(bg[0]), f32::from(bg[1]), f32::from(bg[2])];
    };
    if px[3] == 255 {
        return [f32::from(px[0]), f32::from(px[1]), f32::from(px[2])];
    }
    let alpha = f32::from(px[3]) / 255.0;
    let inv = 1.0 - alpha;
    [
        f32::from(px[0]) * alpha + f32::from(bg[0]) * inv,
        f32::from(px[1]) * alpha + f32::from(bg[1]) * inv,
        f32::from(px[2]) * alpha + f32::from(bg[2]) * inv,
    ]
}

fn preprocess_line_strip(
    rgba: &[u8],
    img_w: usize,
    bg: [u8; 3],
    box_coords: (usize, usize, usize, usize),
) -> TensorValue {
    let (x0, y0, x1, y1) = box_coords;
    let crop_w = x1 - x0 + 1;
    let crop_h = y1 - y0 + 1;

    let raw_w = ((crop_w as f32 / crop_h as f32) * TARGET_LINE_HEIGHT as f32)
        .round()
        .clamp(MIN_LINE_WIDTH, MAX_LINE_WIDTH) as usize;
    let target_w = raw_w.div_ceil(8) * 8;
    let target_h = TARGET_LINE_HEIGHT;

    let plane = target_h * target_w;
    let mut tensor = vec![0.0f32; 3 * plane];

    for dy in 0..target_h {
        let sy_f = if target_h > 1 && crop_h > 1 {
            ((dy as f32 + 0.5) * (crop_h as f32 / target_h as f32) - 0.5)
                .clamp(0.0, (crop_h - 1) as f32)
        } else {
            0.0
        };
        let sy0 = sy_f.floor() as usize;
        let sy1 = (sy0 + 1).min(crop_h - 1);
        let wy = sy_f - sy0 as f32;

        for dx in 0..target_w {
            let sx_f = if target_w > 1 && crop_w > 1 {
                ((dx as f32 + 0.5) * (crop_w as f32 / target_w as f32) - 0.5)
                    .clamp(0.0, (crop_w - 1) as f32)
            } else {
                0.0
            };
            let sx0 = sx_f.floor() as usize;
            let sx1 = (sx0 + 1).min(crop_w - 1);
            let wx = sx_f - sx0 as f32;

            let p00 = sample_rgb_over_bg(rgba, img_w, x0 + sx0, y0 + sy0, bg);
            let p10 = sample_rgb_over_bg(rgba, img_w, x0 + sx1, y0 + sy0, bg);
            let p01 = sample_rgb_over_bg(rgba, img_w, x0 + sx0, y0 + sy1, bg);
            let p11 = sample_rgb_over_bg(rgba, img_w, x0 + sx1, y0 + sy1, bg);

            let dst_idx = dy * target_w + dx;
            for ch in 0..3 {
                let top = p00[ch] * (1.0 - wx) + p10[ch] * wx;
                let bot = p01[ch] * (1.0 - wx) + p11[ch] * wx;
                let val = top * (1.0 - wy) + bot * wy;
                tensor[ch * plane + dst_idx] = ((val / 255.0) - 0.5) / 0.5;
            }
        }
    }

    TensorValue::F32(tensor, vec![1, 3, target_h, target_w])
}

fn ctc_greedy_decode_indices(indices: &[usize], vocab: &[String]) -> String {
    let mut out = String::new();
    let mut prev: Option<usize> = None;
    for &idx in indices {
        if prev != Some(idx)
            && idx != 0
            && let Some(tok) = vocab.get(idx)
        {
            out.push_str(tok);
        }
        prev = Some(idx);
    }
    out.trim().to_string()
}

fn ctc_greedy_decode(logits: &TensorValue, vocab: &[String]) -> Result<String> {
    let (data, shape) = logits.as_f32()?;
    let (timesteps, vocab_size) = match shape {
        [1, t, v] => (*t, *v),
        [t, v] => (*t, *v),
        _ => bail!("Unexpected CTC output tensor shape {shape:?}"),
    };
    if timesteps == 0 || vocab_size == 0 {
        return Ok(String::new());
    }
    anyhow::ensure!(
        data.len() == timesteps * vocab_size,
        "CTC logits length {} != {timesteps}*{vocab_size}",
        data.len()
    );

    let mut indices = Vec::with_capacity(timesteps);
    for step in data.chunks_exact(vocab_size) {
        let mut best_idx = 0usize;
        let mut best_val = f32::NEG_INFINITY;
        for (idx, &val) in step.iter().enumerate() {
            if val > best_val {
                best_val = val;
                best_idx = idx;
            }
        }
        indices.push(best_idx);
    }

    Ok(ctc_greedy_decode_indices(&indices, vocab))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitmap::encode_rgba_png;

    // Minimal protobuf writer helper for building valid synthetic ONNX ModelProto fixtures.
    fn encode_varint(mut v: u64, out: &mut Vec<u8>) {
        loop {
            let b = (v & 0x7F) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                break;
            }
            out.push(b | 0x80);
        }
    }

    fn pb_tag(field: u32, wire: u8, out: &mut Vec<u8>) {
        encode_varint((u64::from(field) << 3) | u64::from(wire), out);
    }

    fn pb_bytes(field: u32, bytes: &[u8], out: &mut Vec<u8>) {
        pb_tag(field, 2, out);
        encode_varint(bytes.len() as u64, out);
        out.extend_from_slice(bytes);
    }

    fn pb_str(field: u32, s: &str, out: &mut Vec<u8>) {
        pb_bytes(field, s.as_bytes(), out);
    }

    fn pb_i64(field: u32, v: i64, out: &mut Vec<u8>) {
        pb_tag(field, 0, out);
        encode_varint(v as u64, out);
    }

    fn pb_f32(field: u32, v: f32, out: &mut Vec<u8>) {
        pb_tag(field, 5, out);
        out.extend_from_slice(&v.to_le_bytes());
    }

    fn pb_tensor_f32(name: &str, dims: &[i64], data: &[f32]) -> Vec<u8> {
        let mut t = Vec::new();
        for &d in dims {
            pb_i64(1, d, &mut t);
        }
        pb_i64(2, 1, &mut t); // FLOAT
        pb_str(8, name, &mut t);
        let mut raw = Vec::with_capacity(data.len() * 4);
        for &f in data {
            raw.extend_from_slice(&f.to_le_bytes());
        }
        pb_bytes(9, &raw, &mut t);
        t
    }

    fn pb_tensor_i64(name: &str, dims: &[i64], data: &[i64]) -> Vec<u8> {
        let mut t = Vec::new();
        for &d in dims {
            pb_i64(1, d, &mut t);
        }
        pb_i64(2, 7, &mut t); // INT64
        pb_str(8, name, &mut t);
        let mut raw = Vec::with_capacity(data.len() * 8);
        for &v in data {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        pb_bytes(9, &raw, &mut t);
        t
    }

    fn pb_attr_ints(name: &str, vals: &[i64]) -> Vec<u8> {
        let mut a = Vec::new();
        pb_str(1, name, &mut a);
        for &v in vals {
            pb_i64(8, v, &mut a);
        }
        pb_i64(20, 7, &mut a); // INTS
        a
    }

    fn pb_attr_int(name: &str, val: i64) -> Vec<u8> {
        let mut a = Vec::new();
        pb_str(1, name, &mut a);
        pb_i64(3, val, &mut a);
        pb_i64(20, 2, &mut a); // INT
        a
    }

    fn pb_attr_f32(name: &str, val: f32) -> Vec<u8> {
        let mut a = Vec::new();
        pb_str(1, name, &mut a);
        pb_f32(2, val, &mut a);
        pb_i64(20, 1, &mut a); // FLOAT
        a
    }

    fn pb_node(op: &str, inputs: &[&str], outputs: &[&str], attrs: Vec<Vec<u8>>) -> Vec<u8> {
        let mut n = Vec::new();
        for inp in inputs {
            pb_str(1, inp, &mut n);
        }
        for out in outputs {
            pb_str(2, out, &mut n);
        }
        pb_str(4, op, &mut n);
        for attr in attrs {
            pb_bytes(5, &attr, &mut n);
        }
        n
    }

    fn pb_value_info(name: &str) -> Vec<u8> {
        let mut v = Vec::new();
        pb_str(1, name, &mut v);
        v
    }

    fn build_synthetic_ctc_onnx_model() -> Vec<u8> {
        // Graph: x [1, 3, 48, W]
        // -> Conv(W=[2, 3, 3, 3], pads=[1,1,1,1], strides=[1, 8]) -> [1, 2, 48, T]
        // -> BatchNormalization(scale=[1,1], B=[0,0], mean=[0,0], var=[1,1])
        // -> HardSwish
        // -> ReduceMean(axes=[2], keepdims=0) -> [1, 2, T]
        // -> Transpose(perm=[0, 2, 1]) -> [1, T, 2]
        // -> Reshape(shape=[-1, 2]) -> [T, 2]
        // -> MatMul(W_fc=[2, 5]) -> [T, 5]  (5 classes: 0=blank, 1=A, 2=B, 3=C, 4=" ")
        // -> Add(b_fc=[5]) -> [T, 5]
        // -> Reshape(shape=[1, -1, 5]) -> [1, T, 5]
        // -> Softmax(axis=-1) -> y [1, T, 5]
        let mut conv_w = vec![0.0f32; 2 * 3 * 3 * 3];
        // Filter 0 responds positively to input; filter 1 slightly negative.
        conv_w[0..27].fill(0.1);
        conv_w[27..54].fill(-0.05);

        let w_fc = [
            // row 0 (from channel 0): boosts class 1 ('A') strongly
            -2.0f32, 5.0, -1.0, -1.0, -1.0, // row 1 (from channel 1)
            0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let b_fc = [0.0f32, 0.5, -1.0, -1.0, -1.0];

        let mut graph = Vec::new();
        pb_bytes(
            1,
            &pb_node(
                "Conv",
                &["x", "conv_w"],
                &["c_out"],
                vec![
                    pb_attr_ints("pads", &[1, 1, 1, 1]),
                    pb_attr_ints("strides", &[1, 8]),
                ],
            ),
            &mut graph,
        );
        pb_bytes(
            1,
            &pb_node(
                "BatchNormalization",
                &["c_out", "bn_scale", "bn_bias", "bn_mean", "bn_var"],
                &["bn_out"],
                vec![pb_attr_f32("epsilon", 1e-5)],
            ),
            &mut graph,
        );
        pb_bytes(
            1,
            &pb_node("HardSwish", &["bn_out"], &["hs_out"], vec![]),
            &mut graph,
        );
        pb_bytes(
            1,
            &pb_node(
                "ReduceMean",
                &["hs_out"],
                &["rm_out"],
                vec![pb_attr_ints("axes", &[2]), pb_attr_int("keepdims", 0)],
            ),
            &mut graph,
        );
        pb_bytes(
            1,
            &pb_node(
                "Transpose",
                &["rm_out"],
                &["tr_out"],
                vec![pb_attr_ints("perm", &[0, 2, 1])],
            ),
            &mut graph,
        );
        pb_bytes(
            1,
            &pb_node("Reshape", &["tr_out", "shape_2d"], &["r2_out"], vec![]),
            &mut graph,
        );
        pb_bytes(
            1,
            &pb_node("MatMul", &["r2_out", "w_fc"], &["mm_out"], vec![]),
            &mut graph,
        );
        pb_bytes(
            1,
            &pb_node("Add", &["mm_out", "b_fc"], &["add_out"], vec![]),
            &mut graph,
        );
        pb_bytes(
            1,
            &pb_node("Reshape", &["add_out", "shape_3d"], &["r3_out"], vec![]),
            &mut graph,
        );
        pb_bytes(
            1,
            &pb_node(
                "Softmax",
                &["r3_out"],
                &["y"],
                vec![pb_attr_int("axis", -1)],
            ),
            &mut graph,
        );

        pb_bytes(
            5,
            &pb_tensor_f32("conv_w", &[2, 3, 3, 3], &conv_w),
            &mut graph,
        );
        pb_bytes(5, &pb_tensor_f32("bn_scale", &[2], &[1.0, 1.0]), &mut graph);
        pb_bytes(5, &pb_tensor_f32("bn_bias", &[2], &[0.0, 0.0]), &mut graph);
        pb_bytes(5, &pb_tensor_f32("bn_mean", &[2], &[0.0, 0.0]), &mut graph);
        pb_bytes(5, &pb_tensor_f32("bn_var", &[2], &[1.0, 1.0]), &mut graph);
        pb_bytes(5, &pb_tensor_i64("shape_2d", &[2], &[-1, 2]), &mut graph);
        pb_bytes(5, &pb_tensor_f32("w_fc", &[2, 5], &w_fc), &mut graph);
        pb_bytes(5, &pb_tensor_f32("b_fc", &[5], &b_fc), &mut graph);
        pb_bytes(5, &pb_tensor_i64("shape_3d", &[3], &[1, -1, 5]), &mut graph);

        pb_bytes(11, &pb_value_info("x"), &mut graph);
        pb_bytes(12, &pb_value_info("y"), &mut graph);

        let mut meta = Vec::new();
        pb_str(1, "character", &mut meta);
        pb_str(2, "A\nB\nC", &mut meta);

        let mut model = Vec::new();
        pb_i64(1, 8, &mut model);
        pb_bytes(7, &graph, &mut model);
        pb_bytes(14, &meta, &mut model);
        model
    }

    #[test]
    fn parses_onnx_metadata_vocab_and_weights() {
        let model_bytes = build_synthetic_ctc_onnx_model();
        let backend = LocalOcrBackend::from_bytes(&model_bytes).unwrap();
        assert_eq!(backend.vocab(), &["", "A", "B", "C", " "]);
        assert_eq!(backend.model.initializers.len(), 9);
        assert_eq!(backend.model.nodes.len(), 10);
    }

    #[test]
    fn ctc_decode_collapses_repeats_and_drops_blanks() {
        let vocab = build_vocab_from_character_dict("A\nB\nC").unwrap();
        let decoded = ctc_greedy_decode_indices(&[0, 1, 1, 0, 2, 4, 3], &vocab);
        assert_eq!(decoded, "AB C");
    }

    #[test]
    fn line_segmentation_splits_two_line_synthetic_png() {
        let w = 160usize;
        let h = 64usize;
        let mut rgba = vec![0u8; w * h * 4];
        for px in rgba.as_chunks_mut::<4>().0 {
            *px = [16, 16, 16, 255];
        }
        // Line 1: y in 8..20 (12px tall)
        for y in 8..20 {
            for x in 20..120 {
                let off = (y * w + x) * 4;
                rgba[off..off + 4].copy_from_slice(&[240, 240, 240, 255]);
            }
        }
        // Gap: y in 20..36 (16px tall > 3px)
        // Line 2: y in 36..48 (12px tall)
        for y in 36..48 {
            for x in 30..110 {
                let off = (y * w + x) * 4;
                rgba[off..off + 4].copy_from_slice(&[240, 240, 240, 255]);
            }
        }

        let bg = estimate_background_rgb(&rgba, w, h);
        assert_eq!(bg, [16, 16, 16]);
        let boxes = segment_text_lines(&rgba, w, h, bg);
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0], (18, 6, 121, 21));
        assert_eq!(boxes[1], (28, 34, 111, 49));
    }

    #[tokio::test]
    async fn local_backend_recognizes_synthetic_cue_and_respects_cancellation() {
        let model_bytes = build_synthetic_ctc_onnx_model();
        let backend = LocalOcrBackend::from_bytes(&model_bytes).unwrap();

        let w = 80u32;
        let h = 32u32;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for px in rgba.as_chunks_mut::<4>().0 {
            *px = [16, 16, 16, 255];
        }
        for y in 8..24 {
            for x in 10..70 {
                let off = ((y * w + x) as usize) * 4;
                rgba[off..off + 4].copy_from_slice(&[250, 250, 250, 255]);
            }
        }
        let png_bytes = encode_rgba_png(&rgba, w, h).unwrap();
        let cue = BitmapSubtitleCue {
            id: 1,
            start_ms: 0,
            end_ms: 1000,
            png_bytes,
            width: w,
            height: h,
        };

        let cancel = CancellationToken::new();
        let recognized = backend
            .recognize_cue(&cue, Some("en"), &cancel)
            .await
            .unwrap();
        assert_eq!(recognized, "A");

        let cancelled = CancellationToken::new();
        cancelled.cancel();
        let err = backend
            .recognize_cue(&cue, None, &cancelled)
            .await
            .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("cancel"));
    }

    #[test]
    fn onnx_tensor_ops_cover_broadcast_slice_concat_gather_split_pool_gemm() {
        let a = TensorValue::F32(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]);
        let b = TensorValue::F32(vec![10.0, 20.0], vec![1, 2]);
        let added = eval_binary(&a, &b, BinaryOp::Add).unwrap();
        assert_eq!(
            added,
            TensorValue::F32(vec![11.0, 22.0, 13.0, 24.0], vec![2, 2])
        );

        let slice_node = ParsedNode {
            inputs: vec![],
            outputs: vec![],
            op_type: "Slice".to_string(),
            attrs: HashMap::new(),
        };
        let sliced = eval_slice(
            &added,
            Some(&TensorValue::I64(vec![0, 1], vec![2])),
            Some(&TensorValue::I64(vec![2, 2], vec![2])),
            Some(&TensorValue::I64(vec![0, 1], vec![2])),
            None,
            &slice_node,
        )
        .unwrap();
        assert_eq!(sliced, TensorValue::F32(vec![22.0, 24.0], vec![2, 1]));

        let mut concat_attrs = HashMap::new();
        concat_attrs.insert(
            "axis".to_string(),
            ParsedAttribute {
                name: "axis".to_string(),
                f: None,
                i: Some(1),
                s: None,
                t: None,
                floats: vec![],
                ints: vec![],
            },
        );
        let concat_node = ParsedNode {
            inputs: vec![],
            outputs: vec![],
            op_type: "Concat".to_string(),
            attrs: concat_attrs,
        };
        let concated = eval_concat(&[&sliced, &sliced], &concat_node).unwrap();
        assert_eq!(
            concated,
            TensorValue::F32(vec![22.0, 22.0, 24.0, 24.0], vec![2, 2])
        );

        let split_node = ParsedNode {
            inputs: vec![],
            outputs: vec!["o0".into(), "o1".into()],
            op_type: "Split".to_string(),
            attrs: concat_node.attrs.clone(),
        };
        let splits = eval_split(&concated, None, &split_node).unwrap();
        assert_eq!(splits.len(), 2);
        assert_eq!(splits[0], sliced);

        let gather_node = ParsedNode {
            inputs: vec![],
            outputs: vec![],
            op_type: "Gather".to_string(),
            attrs: HashMap::new(),
        };
        let gathered = eval_gather(&a, &TensorValue::I64(vec![-1], vec![1]), &gather_node).unwrap();
        assert_eq!(gathered, TensorValue::F32(vec![3.0, 4.0], vec![1, 2]));

        let gemm_node = ParsedNode {
            inputs: vec![],
            outputs: vec![],
            op_type: "Gemm".to_string(),
            attrs: HashMap::new(),
        };
        let gemm_out = eval_gemm(&a, &a, Some(&b), &gemm_node).unwrap();
        assert_eq!(
            gemm_out,
            TensorValue::F32(vec![17.0, 30.0, 25.0, 42.0], vec![2, 2])
        );

        let gap_in = TensorValue::F32(vec![1.0, 3.0, 5.0, 7.0], vec![1, 1, 2, 2]);
        let gap_out = eval_global_average_pool(&gap_in).unwrap();
        assert_eq!(gap_out, TensorValue::F32(vec![4.0], vec![1, 1, 1, 1]));
    }
}
