//! Bitmap subtitle demuxing and decoding for PGS, VobSub, DVB, and XSUB tracks.

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const DEFAULT_CUE_DURATION_MS: u64 = 3500;
const CROP_PADDING_PX: u32 = 12;
const MAX_PGS_OBJECT_PIXELS: usize = 4096 * 4096;
const MAX_VALID_PACKET_DURATION_MS: u64 = 60_000;
const PACKET_GROUP_WINDOW_MS: u64 = 20;

/// Single decoded bitmap subtitle cue rendered as an in-memory PNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitmapSubtitleCue {
    /// 1-based sequential cue index.
    pub id: u32,
    /// Cue start timestamp in milliseconds.
    pub start_ms: u64,
    /// Cue end timestamp in milliseconds.
    pub end_ms: u64,
    /// PNG-encoded RGB/RGBA image bytes cropped to the subtitle bounding box.
    pub png_bytes: Vec<u8>,
    /// Width of [`Self::png_bytes`] in pixels.
    pub width: u32,
    /// Height of [`Self::png_bytes`] in pixels.
    pub height: u32,
}

struct ByteCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn read_u8(&mut self) -> Option<u8> {
        let byte = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(byte)
    }

    fn read_u16_be(&mut self) -> Option<u16> {
        let slice = self.data.get(self.pos..self.pos + 2)?;
        self.pos += 2;
        Some(u16::from_be_bytes([slice[0], slice[1]]))
    }

    fn read_u24_be(&mut self) -> Option<u32> {
        let slice = self.data.get(self.pos..self.pos + 3)?;
        self.pos += 3;
        Some((u32::from(slice[0]) << 16) | (u32::from(slice[1]) << 8) | u32::from(slice[2]))
    }

    fn read_u32_be(&mut self) -> Option<u32> {
        let slice = self.data.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn read_slice(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(len)?;
        let slice = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }
}

#[derive(Debug, Clone)]
struct PgsCompositionObject {
    object_id: u16,
    _window_id: u8,
    crop: Option<(u16, u16, u16, u16)>,
    x: u16,
    y: u16,
}

#[derive(Debug, Clone)]
struct PgsPcs {
    pts_ms: u64,
    _video_width: u16,
    video_height: u16,
    palette_id: u8,
    objects: Vec<PgsCompositionObject>,
}

#[derive(Debug, Clone)]
struct PgsObject {
    width: u16,
    height: u16,
    indices: Vec<u8>,
}

struct InProgressObject {
    width: u16,
    height: u16,
    rle_data: Vec<u8>,
}

struct EpochState {
    palettes: HashMap<u8, [[u8; 4]; 256]>,
    objects: HashMap<u16, PgsObject>,
    in_progress_objects: HashMap<u16, InProgressObject>,
}

impl EpochState {
    fn new() -> Self {
        Self {
            palettes: HashMap::new(),
            objects: HashMap::new(),
            in_progress_objects: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.palettes.clear();
        self.objects.clear();
        self.in_progress_objects.clear();
    }
}

struct OpenCue {
    start_ms: u64,
    end_ms: Option<u64>,
    png_bytes: Vec<u8>,
    width: u32,
    height: u32,
}

fn close_open_cue(
    open_cue: &mut Option<OpenCue>,
    close_at_ms: Option<u64>,
    cues: &mut Vec<BitmapSubtitleCue>,
) -> Result<()> {
    let Some(open) = open_cue.take() else {
        return Ok(());
    };
    let end_ms = match (close_at_ms, open.end_ms) {
        (Some(close_ms), Some(planned)) => planned.min(close_ms).max(open.start_ms + 1),
        (Some(close_ms), None) => close_ms.max(open.start_ms + 1),
        (None, Some(planned)) => planned.max(open.start_ms + 1),
        (None, None) => open.start_ms + DEFAULT_CUE_DURATION_MS,
    };
    let id = u32::try_from(cues.len() + 1).context("Bitmap subtitle cue count overflow")?;
    cues.push(BitmapSubtitleCue {
        id,
        start_ms: open.start_ms,
        end_ms,
        png_bytes: open.png_bytes,
        width: open.width,
        height: open.height,
    });
    Ok(())
}

fn ycbcr_to_rgba(y: u8, cr: u8, cb: u8, alpha: u8, use_bt709: bool) -> [u8; 4] {
    let yf = f64::from(y) - 16.0;
    let cbf = f64::from(cb) - 128.0;
    let crf = f64::from(cr) - 128.0;
    let (r, g, b) = if use_bt709 {
        (
            1.164 * yf + 1.793 * crf,
            1.164 * yf - 0.213 * cbf - 0.533 * crf,
            1.164 * yf + 2.112 * cbf,
        )
    } else {
        (
            1.164 * yf + 1.596 * crf,
            1.164 * yf - 0.392 * cbf - 0.813 * crf,
            1.164 * yf + 2.017 * cbf,
        )
    };
    [
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        b.round().clamp(0.0, 255.0) as u8,
        alpha,
    ]
}

fn parse_pcs_objects(
    payload: &[u8],
    num_objects: usize,
    crop_mask: u8,
) -> Option<Vec<PgsCompositionObject>> {
    let mut cursor = ByteCursor::new(payload);
    let mut objects = Vec::with_capacity(num_objects);
    for _ in 0..num_objects {
        let object_id = cursor.read_u16_be()?;
        let window_id = cursor.read_u8()?;
        let flags = cursor.read_u8()?;
        let x = cursor.read_u16_be()?;
        let y = cursor.read_u16_be()?;
        let crop = if (flags & crop_mask) != 0 {
            let cx = cursor.read_u16_be()?;
            let cy = cursor.read_u16_be()?;
            let cw = cursor.read_u16_be()?;
            let ch = cursor.read_u16_be()?;
            Some((cx, cy, cw, ch))
        } else {
            None
        };
        objects.push(PgsCompositionObject {
            object_id,
            _window_id: window_id,
            crop,
            x,
            y,
        });
    }
    (cursor.remaining() == 0).then_some(objects)
}

fn parse_pcs_segment(pts_ms: u64, payload: &[u8]) -> Result<(PgsPcs, u8)> {
    let mut cursor = ByteCursor::new(payload);
    let video_width = cursor
        .read_u16_be()
        .context("Truncated PGS PCS video width")?;
    let video_height = cursor
        .read_u16_be()
        .context("Truncated PGS PCS video height")?;
    let _frame_rate = cursor.read_u8().context("Truncated PGS PCS frame rate")?;
    let _composition_number = cursor
        .read_u16_be()
        .context("Truncated PGS PCS composition number")?;
    let composition_state = cursor
        .read_u8()
        .context("Truncated PGS PCS composition state")?;
    let _palette_update_flag = cursor
        .read_u8()
        .context("Truncated PGS PCS palette update flag")?;
    let palette_id = cursor.read_u8().context("Truncated PGS PCS palette ID")?;
    let num_objects = usize::from(cursor.read_u8().context("Truncated PGS PCS object count")?);

    let rest = payload.get(cursor.pos..).unwrap_or_default();
    let objects = parse_pcs_objects(rest, num_objects, 0x80)
        .or_else(|| parse_pcs_objects(rest, num_objects, 0x40))
        .context("Invalid PGS PCS composition object list")?;

    Ok((
        PgsPcs {
            pts_ms,
            _video_width: video_width,
            video_height,
            palette_id,
            objects,
        },
        composition_state,
    ))
}

fn parse_pds_segment(payload: &[u8], use_bt709: bool, epoch: &mut EpochState) -> Result<()> {
    let mut cursor = ByteCursor::new(payload);
    let palette_id = cursor.read_u8().context("Truncated PGS PDS palette ID")?;
    let _version = cursor.read_u8().context("Truncated PGS PDS version")?;
    let palette = epoch
        .palettes
        .entry(palette_id)
        .or_insert([[0, 0, 0, 0]; 256]);
    while cursor.remaining() >= 5 {
        let entry_id = cursor.read_u8().context("Missing PDS entry ID")?;
        let y = cursor.read_u8().context("Missing PDS Y")?;
        let cr = cursor.read_u8().context("Missing PDS Cr")?;
        let cb = cursor.read_u8().context("Missing PDS Cb")?;
        let alpha = cursor.read_u8().context("Missing PDS Alpha")?;
        palette[usize::from(entry_id)] = ycbcr_to_rgba(y, cr, cb, alpha, use_bt709);
    }
    Ok(())
}

fn decode_pgs_rle(rle: &[u8], width: u16, height: u16) -> Result<Vec<u8>> {
    let w = usize::from(width);
    let h = usize::from(height);
    let pixel_count = w
        .checked_mul(h)
        .filter(|count| *count <= MAX_PGS_OBJECT_PIXELS)
        .context("PGS object dimensions exceed bounds")?;
    let mut indices = vec![0u8; pixel_count];
    if w == 0 || h == 0 {
        return Ok(indices);
    }

    let mut x = 0usize;
    let mut y = 0usize;
    let mut cursor = ByteCursor::new(rle);

    while let Some(byte) = cursor.read_u8() {
        if byte != 0 {
            if y < h && x < w {
                if let Some(slot) = indices.get_mut(y * w + x) {
                    *slot = byte;
                }
                x += 1;
            }
            continue;
        }

        let Some(flag) = cursor.read_u8() else {
            break;
        };
        if flag == 0 {
            x = 0;
            y += 1;
            continue;
        }

        let two_bytes = (flag & 0x40) != 0;
        let has_color = (flag & 0x80) != 0;
        let run_len = if two_bytes {
            let Some(low) = cursor.read_u8() else {
                break;
            };
            (usize::from(flag & 0x3F) << 8) | usize::from(low)
        } else {
            usize::from(flag & 0x3F)
        };
        let color = if has_color {
            cursor.read_u8().unwrap_or(0)
        } else {
            0
        };

        if y < h && x < w && run_len > 0 {
            let writable = run_len.min(w - x);
            if color != 0 {
                let row_start = y * w + x;
                if let Some(slice) = indices.get_mut(row_start..row_start + writable) {
                    slice.fill(color);
                }
            }
            x += writable;
        }
    }

    Ok(indices)
}

fn parse_ods_segment(payload: &[u8], epoch: &mut EpochState) -> Result<()> {
    let mut cursor = ByteCursor::new(payload);
    let object_id = cursor
        .read_u16_be()
        .context("Truncated PGS ODS object ID")?;
    let _version = cursor.read_u8().context("Truncated PGS ODS version")?;
    let seq_flag = cursor
        .read_u8()
        .context("Truncated PGS ODS sequence flag")?;
    let is_first = (seq_flag & 0x80) != 0;
    let is_last = (seq_flag & 0x40) != 0;

    if is_first {
        let _data_len = cursor
            .read_u24_be()
            .context("Truncated PGS ODS data length")?;
        let width = cursor.read_u16_be().context("Truncated PGS ODS width")?;
        let height = cursor.read_u16_be().context("Truncated PGS ODS height")?;
        let rle_chunk = payload.get(cursor.pos..).unwrap_or_default().to_vec();
        epoch.in_progress_objects.insert(
            object_id,
            InProgressObject {
                width,
                height,
                rle_data: rle_chunk,
            },
        );
    } else if let Some(in_prog) = epoch.in_progress_objects.get_mut(&object_id) {
        let rle_chunk = payload.get(cursor.pos..).unwrap_or_default();
        in_prog.rle_data.extend_from_slice(rle_chunk);
    }

    if is_last && let Some(in_prog) = epoch.in_progress_objects.remove(&object_id) {
        let indices = decode_pgs_rle(&in_prog.rle_data, in_prog.width, in_prog.height)?;
        epoch.objects.insert(
            object_id,
            PgsObject {
                width: in_prog.width,
                height: in_prog.height,
                indices,
            },
        );
    }

    Ok(())
}

pub(crate) fn encode_rgba_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .context("Failed to write PNG header")?;
        writer
            .write_image_data(rgba)
            .context("Failed to write PNG image data")?;
    }
    Ok(png_bytes)
}

fn alpha_blend_over(dst: &mut [u8], src: [u8; 4]) {
    let a = u16::from(src[3]);
    if a == 0 {
        return;
    }
    if a == 255 {
        dst[0] = src[0];
        dst[1] = src[1];
        dst[2] = src[2];
        dst[3] = 255;
        return;
    }
    let inv = 255 - a;
    dst[0] = ((u16::from(src[0]) * a + u16::from(dst[0]) * inv + 127) / 255) as u8;
    dst[1] = ((u16::from(src[1]) * a + u16::from(dst[1]) * inv + 127) / 255) as u8;
    dst[2] = ((u16::from(src[2]) * a + u16::from(dst[2]) * inv + 127) / 255) as u8;
    dst[3] = 255;
}

fn render_pcs_cue(pcs: &PgsPcs, epoch: &EpochState) -> Result<Option<(Vec<u8>, u32, u32)>> {
    if pcs.objects.is_empty() {
        return Ok(None);
    }
    let Some(palette) = epoch.palettes.get(&pcs.palette_id) else {
        return Ok(None);
    };

    // First pass: find tight bounding box of all non-transparent pixels across objects.
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut has_visible_pixel = false;

    for comp in &pcs.objects {
        let Some(obj) = epoch.objects.get(&comp.object_id) else {
            continue;
        };
        let (src_x, src_y, src_w, src_h) = resolve_object_crop(obj, comp.crop);
        let obj_w = u32::from(obj.width);
        let base_x = u32::from(comp.x);
        let base_y = u32::from(comp.y);

        for dy in 0..src_h {
            for dx in 0..src_w {
                let idx_pos = ((src_y + dy) * obj_w + (src_x + dx)) as usize;
                let Some(&pal_idx) = obj.indices.get(idx_pos) else {
                    continue;
                };
                if palette[usize::from(pal_idx)][3] == 0 {
                    continue;
                }
                let gx = base_x + dx;
                let gy = base_y + dy;
                min_x = min_x.min(gx);
                min_y = min_y.min(gy);
                max_x = max_x.max(gx);
                max_y = max_y.max(gy);
                has_visible_pixel = true;
            }
        }
    }

    if !has_visible_pixel {
        return Ok(None);
    }

    let content_w = max_x - min_x + 1;
    let content_h = max_y - min_y + 1;
    let out_w = content_w + CROP_PADDING_PX * 2;
    let out_h = content_h + CROP_PADDING_PX * 2;
    let total_pixels = (out_w as usize)
        .checked_mul(out_h as usize)
        .filter(|n| *n <= MAX_PGS_OBJECT_PIXELS)
        .context("PGS rendered cue dimensions exceed bounds")?;

    let mut rgba = Vec::with_capacity(total_pixels * 4);
    for _ in 0..total_pixels {
        rgba.extend_from_slice(&[16, 16, 16, 255]);
    }

    for comp in &pcs.objects {
        let Some(obj) = epoch.objects.get(&comp.object_id) else {
            continue;
        };
        let (src_x, src_y, src_w, src_h) = resolve_object_crop(obj, comp.crop);
        let obj_w = u32::from(obj.width);
        let base_x = u32::from(comp.x);
        let base_y = u32::from(comp.y);

        for dy in 0..src_h {
            for dx in 0..src_w {
                let idx_pos = ((src_y + dy) * obj_w + (src_x + dx)) as usize;
                let Some(&pal_idx) = obj.indices.get(idx_pos) else {
                    continue;
                };
                let src_px = palette[usize::from(pal_idx)];
                if src_px[3] == 0 {
                    continue;
                }
                let gx = base_x + dx;
                let gy = base_y + dy;
                if gx < min_x || gx > max_x || gy < min_y || gy > max_y {
                    continue;
                }
                let out_x = (gx - min_x) + CROP_PADDING_PX;
                let out_y = (gy - min_y) + CROP_PADDING_PX;
                let dst_offset = ((out_y * out_w + out_x) * 4) as usize;
                if let Some(dst) = rgba.get_mut(dst_offset..dst_offset + 4) {
                    alpha_blend_over(dst, src_px);
                }
            }
        }
    }

    let png_bytes = encode_rgba_png(&rgba, out_w, out_h)?;
    Ok(Some((png_bytes, out_w, out_h)))
}

fn resolve_object_crop(
    obj: &PgsObject,
    crop: Option<(u16, u16, u16, u16)>,
) -> (u32, u32, u32, u32) {
    let obj_w = u32::from(obj.width);
    let obj_h = u32::from(obj.height);
    match crop {
        Some((cx, cy, cw, ch)) => {
            let sx = u32::from(cx).min(obj_w);
            let sy = u32::from(cy).min(obj_h);
            let sw = u32::from(cw).min(obj_w - sx);
            let sh = u32::from(ch).min(obj_h - sy);
            (sx, sy, sw, sh)
        }
        None => (0, 0, obj_w, obj_h),
    }
}

/// Parses a raw Blu-ray PGS (`.sup`) byte stream into cropped PNG subtitle cues.
pub fn parse_pgs_sup(bytes: &[u8], limit: Option<usize>) -> Result<Vec<BitmapSubtitleCue>> {
    if bytes.is_empty() || limit == Some(0) {
        return Ok(Vec::new());
    }

    let mut cursor = ByteCursor::new(bytes);
    let mut epoch = EpochState::new();
    let mut pending_pcs: Option<PgsPcs> = None;
    let mut open_cue: Option<OpenCue> = None;
    let mut cues = Vec::new();
    let mut last_video_height: u16 = 1080;

    while cursor.remaining() > 0 {
        let offset = cursor.pos;
        let magic = cursor
            .read_slice(2)
            .with_context(|| format!("Truncated PGS segment header at offset {offset}"))?;
        if magic != b"PG" {
            bail!("Invalid PGS segment magic at offset {offset}");
        }
        let pts = cursor
            .read_u32_be()
            .with_context(|| format!("Truncated PGS PTS at offset {offset}"))?;
        let _dts = cursor
            .read_u32_be()
            .with_context(|| format!("Truncated PGS DTS at offset {offset}"))?;
        let seg_type = cursor
            .read_u8()
            .with_context(|| format!("Truncated PGS segment type at offset {offset}"))?;
        let seg_len = usize::from(
            cursor
                .read_u16_be()
                .with_context(|| format!("Truncated PGS segment length at offset {offset}"))?,
        );
        let payload = cursor.read_slice(seg_len).with_context(|| {
            format!("Truncated PGS segment payload at offset {offset} (expected {seg_len} bytes)")
        })?;
        let pts_ms = (u64::from(pts) * 1000) / 90_000;

        match seg_type {
            0x16 => {
                let (pcs, comp_state) = parse_pcs_segment(pts_ms, payload)?;
                if comp_state == 0x80 {
                    epoch.clear();
                }
                if pcs.video_height > 0 {
                    last_video_height = pcs.video_height;
                }
                pending_pcs = Some(pcs);
            }
            0x14 => {
                let height = pending_pcs
                    .as_ref()
                    .map(|p| p.video_height)
                    .unwrap_or(last_video_height);
                let use_bt709 = height > 576 || height == 0;
                parse_pds_segment(payload, use_bt709, &mut epoch)?;
            }
            0x15 => {
                parse_ods_segment(payload, &mut epoch)?;
            }
            0x17 => {
                // Window Definition Segment; object placement already carries x/y/crop in PCS.
            }
            0x80 => {
                if let Some(pcs) = pending_pcs.take() {
                    if let Some((png_bytes, width, height)) = render_pcs_cue(&pcs, &epoch)? {
                        close_open_cue(&mut open_cue, Some(pcs.pts_ms), &mut cues)?;
                        if limit.is_some_and(|max| cues.len() >= max) {
                            return Ok(cues);
                        }
                        open_cue = Some(OpenCue {
                            start_ms: pcs.pts_ms,
                            end_ms: None,
                            png_bytes,
                            width,
                            height,
                        });
                    } else {
                        close_open_cue(&mut open_cue, Some(pcs.pts_ms), &mut cues)?;
                        if limit.is_some_and(|max| cues.len() >= max) {
                            return Ok(cues);
                        }
                    }
                }
            }
            other => {
                bail!("Unsupported PGS segment type 0x{other:02X} at offset {offset}");
            }
        }
    }

    close_open_cue(&mut open_cue, None, &mut cues)?;
    Ok(cues)
}

pub(crate) async fn run_cancellable_output(
    mut command: tokio::process::Command,
    timeout: Duration,
    cancel: Option<&CancellationToken>,
    context_label: &str,
) -> Result<std::process::Output> {
    #[cfg(windows)]
    command.creation_flags(0x0800_4000);
    command.kill_on_drop(true).stdin(Stdio::null());

    let fut = tokio::time::timeout(timeout, async {
        let mut attempts = 0;
        loop {
            match command.output().await {
                Err(e) if e.raw_os_error() == Some(26) && attempts < 10 => {
                    attempts += 1;
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                res => break res,
            }
        }
    });
    let output = if let Some(token) = cancel {
        tokio::select! {
            biased;
            _ = token.cancelled() => bail!("Bitmap subtitle extraction cancelled"),
            res = fut => res,
        }
    } else {
        fut.await
    };

    output
        .with_context(|| format!("{context_label} timed out"))?
        .with_context(|| format!("Failed to execute {context_label}"))
}

#[derive(Debug, Deserialize)]
struct FfprobePacketProbe {
    streams: Option<Vec<FfprobeStreamInfo>>,
    format: Option<FfprobeFormatInfo>,
    packets: Option<Vec<FfprobePacketEntry>>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStreamInfo {
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormatInfo {
    start_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobePacketEntry {
    pts_time: Option<String>,
    duration_time: Option<String>,
    #[allow(dead_code)]
    size: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SubtitlePacketEvent {
    pts_ms: u64,
    duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PacketProbeResult {
    stream_dims: Option<(u32, u32)>,
    events: Vec<SubtitlePacketEvent>,
}

fn parse_ffprobe_packet_probe(json_bytes: &[u8]) -> Result<PacketProbeResult> {
    let probe: FfprobePacketProbe =
        serde_json::from_slice(json_bytes).context("Failed to parse ffprobe packet JSON")?;
    let stream_dims = probe
        .streams
        .as_deref()
        .and_then(|streams| streams.first())
        .and_then(|s| match (s.width, s.height) {
            (Some(w), Some(h)) if (1..=4096).contains(&w) && (1..=4096).contains(&h) => {
                Some((w, h))
            }
            _ => None,
        });
    let format_start_sec = probe
        .format
        .and_then(|f| f.start_time)
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|t| t.is_finite() && *t >= 0.0)
        .unwrap_or(0.0);

    let mut raw_events = Vec::new();
    for pkt in probe.packets.unwrap_or_default() {
        let Some(pts_sec) = pkt
            .pts_time
            .as_deref()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|t| t.is_finite())
        else {
            continue;
        };
        let rel_sec = (pts_sec - format_start_sec).max(0.0);
        let pts_ms = (rel_sec * 1000.0).round() as u64;
        let duration_ms = pkt
            .duration_time
            .as_deref()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|d| d.is_finite() && *d > 0.0)
            .map(|d| (d * 1000.0).round() as u64)
            .filter(|ms| *ms > 0 && *ms <= MAX_VALID_PACKET_DURATION_MS);
        raw_events.push(SubtitlePacketEvent {
            pts_ms,
            duration_ms,
        });
    }

    raw_events.sort_by_key(|evt| evt.pts_ms);

    let mut grouped: Vec<SubtitlePacketEvent> = Vec::new();
    for evt in raw_events {
        if let Some(last) = grouped.last_mut()
            && evt.pts_ms.saturating_sub(last.pts_ms) <= PACKET_GROUP_WINDOW_MS
        {
            last.duration_ms = match (last.duration_ms, evt.duration_ms) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
            continue;
        }
        grouped.push(evt);
    }

    Ok(PacketProbeResult {
        stream_dims,
        events: grouped,
    })
}

#[cfg_attr(not(test), allow(dead_code))]
fn parse_ffprobe_packet_events(json_bytes: &[u8]) -> Result<Vec<SubtitlePacketEvent>> {
    parse_ffprobe_packet_probe(json_bytes).map(|probe| probe.events)
}

/// Decodes PNG bytes into tightly packed RGBA8 pixels (`width * height * 4`).
pub(crate) fn decode_png_to_rgba(png_bytes: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = decoder
        .read_info()
        .context("Failed to read PNG image header")?;
    let buf_size = reader
        .output_buffer_size()
        .context("PNG image exceeds decoder buffer limits")?;
    let mut buf = vec![0u8; buf_size];
    let info = reader
        .next_frame(&mut buf)
        .context("Failed to decode PNG frame")?;
    let frame = buf
        .get(..info.buffer_size())
        .context("Decoded PNG buffer smaller than expected")?;
    let width = info.width;
    let height = info.height;
    if width == 0 || height == 0 {
        return Ok((Vec::new(), width, height));
    }

    let bpp = match info.color_type {
        png::ColorType::Rgba => 4usize,
        png::ColorType::Rgb => 3usize,
        png::ColorType::GrayscaleAlpha => 2usize,
        png::ColorType::Grayscale => 1usize,
        other => bail!("Unexpected decoded PNG color type: {other:?}"),
    };

    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .context("PNG dimensions overflow pixel count")?;
    let mut rgba_full = Vec::with_capacity(pixel_count * 4);
    for idx in 0..pixel_count {
        let off = idx * bpp;
        let px = match info.color_type {
            png::ColorType::Rgba => {
                let s = frame.get(off..off + 4).context("Truncated RGBA frame")?;
                [s[0], s[1], s[2], s[3]]
            }
            png::ColorType::Rgb => {
                let s = frame.get(off..off + 3).context("Truncated RGB frame")?;
                [s[0], s[1], s[2], 255]
            }
            png::ColorType::GrayscaleAlpha => {
                let s = frame.get(off..off + 2).context("Truncated GA frame")?;
                [s[0], s[0], s[0], s[1]]
            }
            png::ColorType::Grayscale => {
                let g = *frame.get(off).context("Truncated grayscale frame")?;
                [g, g, g, 255]
            }
            other => bail!("Unexpected decoded PNG color type: {other:?}"),
        };
        rgba_full.extend_from_slice(&px);
    }

    Ok((rgba_full, width, height))
}

fn crop_png_to_rgba_cue(png_bytes: &[u8]) -> Result<Option<(Vec<u8>, u32, u32)>> {
    if png_bytes.is_empty() {
        return Ok(None);
    }

    let (rgba_full, width, height) = decode_png_to_rgba(png_bytes)?;
    if width == 0 || height == 0 {
        return Ok(None);
    }

    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut has_content = false;

    for y in 0..height {
        for x in 0..width {
            let off = ((y * width + x) as usize) * 4;
            let px = rgba_full
                .get(off..off + 4)
                .context("Truncated RGBA buffer")?;
            if px[3] > 0 && (px[0] > 8 || px[1] > 8 || px[2] > 8) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                has_content = true;
            }
        }
    }

    if !has_content {
        return Ok(None);
    }

    let x0 = min_x.saturating_sub(CROP_PADDING_PX);
    let y0 = min_y.saturating_sub(CROP_PADDING_PX);
    let x1 = (max_x + CROP_PADDING_PX).min(width - 1);
    let y1 = (max_y + CROP_PADDING_PX).min(height - 1);
    let crop_w = x1 - x0 + 1;
    let crop_h = y1 - y0 + 1;

    let mut cropped = Vec::with_capacity((crop_w as usize) * (crop_h as usize) * 4);
    for y in y0..=y1 {
        let start = ((y * width + x0) * 4) as usize;
        let len = (crop_w * 4) as usize;
        let row = rgba_full
            .get(start..start + len)
            .context("Row slice out of bounds while cropping PNG")?;
        cropped.extend_from_slice(row);
    }

    let cropped_png = encode_rgba_png(&cropped, crop_w, crop_h)?;
    Ok(Some((cropped_png, crop_w, crop_h)))
}

async fn extract_pgs_from_container(
    ffmpeg: &str,
    input: &Path,
    stream_index: u32,
    limit: Option<usize>,
    cancel: Option<&CancellationToken>,
) -> Result<Vec<BitmapSubtitleCue>> {
    let temp_dir = tempfile::tempdir().context("Failed to create temp dir for PGS extraction")?;
    let sup_path = temp_dir.path().join("track.sup");

    let mut cmd = tokio::process::Command::new(ffmpeg);
    cmd.args(["-nostdin", "-v", "error", "-i"])
        .arg(input)
        .args([
            "-map",
            &format!("0:{stream_index}"),
            "-c:s",
            "copy",
            "-f",
            "sup",
            "-y",
        ])
        .arg(&sup_path);

    let output =
        run_cancellable_output(cmd, Duration::from_secs(120), cancel, "ffmpeg PGS demux").await?;
    if !output.status.success() {
        bail!(
            "Failed to demux PGS subtitle stream {stream_index}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let bytes = tokio::fs::read(&sup_path)
        .await
        .context("Failed to read extracted PGS stream")?;
    parse_pgs_sup(&bytes, limit)
}

async fn extract_rendered_bitmap_cues(
    ffmpeg: &str,
    ffprobe: &str,
    input: &Path,
    stream_index: u32,
    limit: Option<usize>,
    cancel: Option<&CancellationToken>,
) -> Result<Vec<BitmapSubtitleCue>> {
    let mut probe_cmd = tokio::process::Command::new(ffprobe);
    probe_cmd
        .args([
            "-v",
            "error",
            "-select_streams",
            &stream_index.to_string(),
            "-show_streams",
            "-show_format",
            "-show_packets",
            "-show_entries",
            "format=start_time:stream=width,height:packet=pts_time,duration_time,size",
            "-of",
            "json",
        ])
        .arg(input);

    let probe_output = run_cancellable_output(
        probe_cmd,
        Duration::from_secs(60),
        cancel,
        "ffprobe subtitle packets",
    )
    .await?;
    if !probe_output.status.success() {
        bail!(
            "Failed to inspect subtitle packets for stream {stream_index}: {}",
            String::from_utf8_lossy(&probe_output.stderr)
        );
    }

    let PacketProbeResult {
        stream_dims,
        events,
    } = parse_ffprobe_packet_probe(&probe_output.stdout)?;
    if events.is_empty() || limit == Some(0) {
        return Ok(Vec::new());
    }

    let (canvas_size, filter) = if let Some((w, h)) = stream_dims {
        (
            format!("{w}x{h}"),
            format!("[1:v][0:{stream_index}]overlay=eof_action=pass[v]"),
        )
    } else {
        (
            "1280x720".to_string(),
            format!(
                "[0:{stream_index}]scale=1280:720:force_original_aspect_ratio=decrease[s];[1:v][s]overlay=(W-w)/2:(H-h)/2:eof_action=pass[v]"
            ),
        )
    };
    let concurrency = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 6);
    let chunk_size = limit
        .map(|max| max.clamp(1, concurrency * 2))
        .unwrap_or(concurrency * 2);
    let sem = Arc::new(tokio::sync::Semaphore::new(concurrency));

    let mut cues = Vec::new();
    let mut open_cue: Option<OpenCue> = None;

    for chunk in events.chunks(chunk_size) {
        if cancel.is_some_and(CancellationToken::is_cancelled) {
            bail!("Bitmap subtitle extraction cancelled");
        }

        let mut join_set = tokio::task::JoinSet::new();
        for (idx, evt) in chunk.iter().cloned().enumerate() {
            let sem = sem.clone();
            let ffmpeg = ffmpeg.to_string();
            let input_path = input.to_path_buf();
            let canvas_size = canvas_size.clone();
            let filter = filter.clone();
            let cancel_tok = cancel.cloned();

            join_set.spawn(async move {
                let _permit = sem
                    .acquire_owned()
                    .await
                    .context("Bitmap render semaphore closed")?;
                if cancel_tok
                    .as_ref()
                    .is_some_and(CancellationToken::is_cancelled)
                {
                    bail!("Bitmap subtitle extraction cancelled");
                }

                let rel_pts_sec = evt.pts_ms as f64 / 1000.0;
                let seek_sec = (rel_pts_sec - 0.002).max(0.0);
                let out_offset = (rel_pts_sec - seek_sec) + 0.05;
                let lavfi_duration = out_offset + 0.20;

                let mut render_cmd = tokio::process::Command::new(&ffmpeg);
                render_cmd
                    .args([
                        "-nostdin",
                        "-v",
                        "error",
                        "-ss",
                        &format!("{seek_sec:.6}"),
                        "-i",
                    ])
                    .arg(&input_path)
                    .args([
                        "-f",
                        "lavfi",
                        "-i",
                        &format!("color=c=black:s={canvas_size}:d={lavfi_duration:.3}"),
                        "-filter_complex",
                        &filter,
                        "-map",
                        "[v]",
                        "-ss",
                        &format!("{out_offset:.3}"),
                        "-frames:v",
                        "1",
                        "-f",
                        "image2pipe",
                        "-vcodec",
                        "png",
                        "-",
                    ]);

                let out = run_cancellable_output(
                    render_cmd,
                    Duration::from_secs(30),
                    cancel_tok.as_ref(),
                    "ffmpeg bitmap cue render",
                )
                .await?;

                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    if stderr.trim().is_empty() {
                        return anyhow::Ok((idx, evt, None));
                    }
                    bail!(
                        "Failed to render bitmap subtitle cue at {}ms: {}",
                        evt.pts_ms,
                        stderr
                    );
                }

                let stdout = out.stdout;
                let cropped = tokio::task::spawn_blocking(move || crop_png_to_rgba_cue(&stdout))
                    .await
                    .context("Bitmap crop worker panicked")??;
                anyhow::Ok((idx, evt, cropped))
            });
        }

        let mut chunk_results = Vec::with_capacity(chunk.len());
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(Ok(item)) => chunk_results.push(item),
                Ok(Err(err)) => {
                    join_set.abort_all();
                    return Err(err);
                }
                Err(err) => {
                    join_set.abort_all();
                    return Err(err).context("Bitmap cue render task failed");
                }
            }
        }
        chunk_results.sort_by_key(|(idx, _, _)| *idx);

        for (_, evt, cropped) in chunk_results {
            if let Some((png_bytes, width, height)) = cropped {
                close_open_cue(&mut open_cue, Some(evt.pts_ms), &mut cues)?;
                if limit.is_some_and(|max| cues.len() >= max) {
                    return Ok(cues);
                }
                let planned_end = evt
                    .duration_ms
                    .map(|dur| evt.pts_ms + dur)
                    .unwrap_or(evt.pts_ms + DEFAULT_CUE_DURATION_MS);
                open_cue = Some(OpenCue {
                    start_ms: evt.pts_ms,
                    end_ms: Some(planned_end),
                    png_bytes,
                    width,
                    height,
                });
            } else {
                close_open_cue(&mut open_cue, Some(evt.pts_ms), &mut cues)?;
                if limit.is_some_and(|max| cues.len() >= max) {
                    return Ok(cues);
                }
            }
        }
    }

    close_open_cue(&mut open_cue, None, &mut cues)?;
    Ok(cues)
}

/// Extracts and decodes bitmap subtitle cues from a media container track.
///
/// Supports `hdmv_pgs_subtitle` (via raw `.sup` demux + pure-Rust parsing) as well as
/// `dvd_subtitle`, `dvb_subtitle`, `xsub`, and other FFmpeg-decodable bitmap tracks.
pub async fn extract_bitmap_subtitles(
    ffmpeg: &str,
    ffprobe: &str,
    input: &Path,
    stream_index: u32,
    limit: Option<usize>,
    cancel: Option<&CancellationToken>,
) -> Result<Vec<BitmapSubtitleCue>> {
    if cancel.is_some_and(CancellationToken::is_cancelled) {
        bail!("Bitmap subtitle extraction cancelled");
    }

    let tracks = if let Some(token) = cancel {
        tokio::select! {
            biased;
            _ = token.cancelled() => bail!("Bitmap subtitle extraction cancelled"),
            res = srt_extract::embedded::list_embedded_subtitles(ffprobe, input) => res?,
        }
    } else {
        srt_extract::embedded::list_embedded_subtitles(ffprobe, input).await?
    };

    let track = tracks
        .iter()
        .find(|t| t.index == stream_index)
        .with_context(|| format!("Subtitle track {stream_index} not found"))?;

    if track.text_based {
        bail!(
            "Subtitle track {} ({}) is text-based; extract it directly as text instead of running OCR",
            track.index,
            track.codec
        );
    }

    if limit == Some(0) {
        return Ok(Vec::new());
    }

    if track.codec == "hdmv_pgs_subtitle" {
        extract_pgs_from_container(ffmpeg, input, stream_index, limit, cancel).await
    } else {
        extract_rendered_bitmap_cues(ffmpeg, ffprobe, input, stream_index, limit, cancel).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pgs_segment(pts_ms: u64, seg_type: u8, payload: &[u8]) -> Vec<u8> {
        let pts_90k = ((pts_ms * 90_000) / 1000) as u32;
        let mut out = Vec::with_capacity(13 + payload.len());
        out.extend_from_slice(b"PG");
        out.extend_from_slice(&pts_90k.to_be_bytes());
        out.extend_from_slice(&pts_90k.to_be_bytes());
        out.push(seg_type);
        out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    type TestCropRect = (u16, u16, u16, u16);

    #[derive(Clone, Copy)]
    struct TestPcsObject {
        obj_id: u16,
        win_id: u8,
        flags: u8,
        x: u16,
        y: u16,
        crop: Option<TestCropRect>,
    }

    fn build_pcs(comp_state: u8, palette_id: u8, objects: &[TestPcsObject]) -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(&1280u16.to_be_bytes());
        p.extend_from_slice(&720u16.to_be_bytes());
        p.push(0x10);
        p.extend_from_slice(&1u16.to_be_bytes());
        p.push(comp_state);
        p.push(0x00);
        p.push(palette_id);
        p.push(objects.len() as u8);
        for obj in objects {
            p.extend_from_slice(&obj.obj_id.to_be_bytes());
            p.push(obj.win_id);
            p.push(obj.flags);
            p.extend_from_slice(&obj.x.to_be_bytes());
            p.extend_from_slice(&obj.y.to_be_bytes());
            if let Some((cx, cy, cw, ch)) = obj.crop {
                p.extend_from_slice(&cx.to_be_bytes());
                p.extend_from_slice(&cy.to_be_bytes());
                p.extend_from_slice(&cw.to_be_bytes());
                p.extend_from_slice(&ch.to_be_bytes());
            }
        }
        p
    }

    fn build_pds(palette_id: u8, entries: &[(u8, u8, u8, u8, u8)]) -> Vec<u8> {
        let mut p = vec![palette_id, 0];
        for &(id, y, cr, cb, a) in entries {
            p.extend_from_slice(&[id, y, cr, cb, a]);
        }
        p
    }

    fn build_rle_solid_rect(w: u16, h: u16, color: u8) -> Vec<u8> {
        assert!(w <= 63);
        let mut rle = Vec::new();
        for _ in 0..h {
            rle.push(0x00);
            rle.push(0x80 | (w as u8));
            rle.push(color);
            rle.push(0x00);
            rle.push(0x00);
        }
        rle
    }

    fn build_fragmented_ods(obj_id: u16, w: u16, h: u16, rle: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let split = rle.len() / 2;
        let obj_data_len = (4 + rle.len()) as u32;
        let mut first = Vec::new();
        first.extend_from_slice(&obj_id.to_be_bytes());
        first.push(0);
        first.push(0x80); // First in sequence
        first.push(((obj_data_len >> 16) & 0xFF) as u8);
        first.push(((obj_data_len >> 8) & 0xFF) as u8);
        first.push((obj_data_len & 0xFF) as u8);
        first.extend_from_slice(&w.to_be_bytes());
        first.extend_from_slice(&h.to_be_bytes());
        first.extend_from_slice(&rle[..split]);

        let mut second = Vec::new();
        second.extend_from_slice(&obj_id.to_be_bytes());
        second.push(0);
        second.push(0x40); // Last in sequence
        second.extend_from_slice(&rle[split..]);
        (first, second)
    }

    fn build_synthetic_pgs_sup() -> Vec<u8> {
        let mut sup = Vec::new();

        // Cue 1 at 1000ms..2500ms: 30x12 white box, fragmented across 2 ODS segments.
        let rle1 = build_rle_solid_rect(30, 12, 1);
        let (ods1_a, ods1_b) = build_fragmented_ods(0, 30, 12, &rle1);
        sup.extend(pgs_segment(
            1000,
            0x16,
            &build_pcs(
                0x80,
                0,
                &[TestPcsObject {
                    obj_id: 0,
                    win_id: 0,
                    flags: 0x00,
                    x: 100,
                    y: 200,
                    crop: None,
                }],
            ),
        ));
        sup.extend(pgs_segment(
            1000,
            0x14,
            &build_pds(0, &[(0, 16, 128, 128, 0), (1, 235, 128, 128, 255)]),
        ));
        sup.extend(pgs_segment(1000, 0x15, &ods1_a));
        sup.extend(pgs_segment(1000, 0x15, &ods1_b));
        sup.extend(pgs_segment(1000, 0x80, &[]));

        // Clear Display Set at 2500ms.
        sup.extend(pgs_segment(2500, 0x16, &build_pcs(0x00, 0, &[])));
        sup.extend(pgs_segment(2500, 0x80, &[]));

        // Cue 2 at 4000ms with crop (10x6 cropped out of 30x12), no clear at EOF -> +3500ms.
        sup.extend(pgs_segment(
            4000,
            0x16,
            &build_pcs(
                0x40,
                0,
                &[TestPcsObject {
                    obj_id: 0,
                    win_id: 0,
                    flags: 0x80,
                    x: 300,
                    y: 500,
                    crop: Some((5, 3, 10, 6)),
                }],
            ),
        ));
        sup.extend(pgs_segment(4000, 0x80, &[]));

        sup
    }

    #[test]
    fn parse_pgs_sup_decodes_fragmented_ods_crop_and_timings() {
        let sup = build_synthetic_pgs_sup();
        let cues = parse_pgs_sup(&sup, None).unwrap();
        assert_eq!(cues.len(), 2);

        assert_eq!(cues[0].id, 1);
        assert_eq!(cues[0].start_ms, 1000);
        assert_eq!(cues[0].end_ms, 2500);
        assert_eq!(cues[0].width, 30 + CROP_PADDING_PX * 2);
        assert_eq!(cues[0].height, 12 + CROP_PADDING_PX * 2);
        assert!(cues[0].png_bytes.starts_with(b"\x89PNG"));

        // Verify dark opaque background + white interior pixel via png decoder.
        let decoder = png::Decoder::new(Cursor::new(&cues[0].png_bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0u8; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        // Corner pixel (0, 0) must be opaque dark [16, 16, 16, 255].
        assert_eq!(&buf[0..4], &[16, 16, 16, 255]);
        // Center pixel inside padding should be near-white [235, 235, 235, 255].
        let center_off = ((14 * info.width + 14) * 4) as usize;
        assert!(buf[center_off] >= 230 && buf[center_off + 3] == 255);

        assert_eq!(cues[1].id, 2);
        assert_eq!(cues[1].start_ms, 4000);
        assert_eq!(cues[1].end_ms, 7500);
        assert_eq!(cues[1].width, 10 + CROP_PADDING_PX * 2);
        assert_eq!(cues[1].height, 6 + CROP_PADDING_PX * 2);
    }

    #[test]
    fn parse_pgs_sup_honors_limit_and_rejects_corrupt_magic() {
        let sup = build_synthetic_pgs_sup();
        let limited = parse_pgs_sup(&sup, Some(1)).unwrap();
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].start_ms, 1000);
        assert_eq!(limited[0].end_ms, 2500);

        assert!(parse_pgs_sup(&sup, Some(0)).unwrap().is_empty());
        assert!(parse_pgs_sup(&[], None).unwrap().is_empty());
        assert!(parse_pgs_sup(b"BAD_MAGIC_123456", None).is_err());
    }

    #[test]
    fn parse_ffprobe_packet_events_groups_and_clamps_sentinel_durations() {
        let json = br#"{
            "format": {"start_time": "1.500000"},
            "packets": [
                {"pts_time": "1.500000", "duration_time": "4294967.295000", "size": "114"},
                {"pts_time": "1.510000", "duration_time": "0.000000", "size": "25"},
                {"pts_time": "3.500000", "duration_time": "1.250000", "size": "1200"}
            ]
        }"#;
        let events = parse_ffprobe_packet_events(json).unwrap();
        assert_eq!(
            events,
            vec![
                SubtitlePacketEvent {
                    pts_ms: 0,
                    duration_ms: None
                },
                SubtitlePacketEvent {
                    pts_ms: 2000,
                    duration_ms: Some(1250)
                }
            ]
        );
    }

    #[tokio::test]
    async fn extract_bitmap_subtitles_handles_pgs_dvdsub_and_rejects_text_tracks() {
        let dir = tempfile::tempdir().unwrap();
        let sup_path = dir.path().join("sub.sup");
        std::fs::write(&sup_path, build_synthetic_pgs_sup()).unwrap();

        // 1. PGS-in-MKV
        let pgs_mkv = dir.path().join("pgs.mkv");
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-copyts",
                "-f",
                "lavfi",
                "-i",
                "color=size=1280x720:duration=6:rate=1",
                "-i",
            ])
            .arg(&sup_path)
            .args([
                "-map",
                "0:v",
                "-map",
                "1:s",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-c:s",
                "copy",
                "-y",
            ])
            .arg(&pgs_mkv)
            .status()
            .expect("ffmpeg is required for bitmap integration test");
        assert!(status.success());

        let pgs_cues = extract_bitmap_subtitles("ffmpeg", "ffprobe", &pgs_mkv, 1, None, None)
            .await
            .unwrap();
        assert_eq!(pgs_cues.len(), 2);
        assert_eq!(pgs_cues[0].end_ms - pgs_cues[0].start_ms, 1500);

        // 2. DVD subtitle (dvdsub) in MKV (single clean cue with WDS so ffmpeg pgssub->dvdsub has no warnings)
        let mut dvd_sup = Vec::new();
        let rle = build_rle_solid_rect(40, 16, 1);
        let (o1, o2) = build_fragmented_ods(0, 40, 16, &rle);
        dvd_sup.extend(pgs_segment(
            1000,
            0x16,
            &build_pcs(
                0x80,
                0,
                &[TestPcsObject {
                    obj_id: 0,
                    win_id: 0,
                    flags: 0x00,
                    x: 100,
                    y: 200,
                    crop: None,
                }],
            ),
        ));
        dvd_sup.extend(pgs_segment(
            1000,
            0x17,
            &[1, 0, 0, 100, 0, 200, 0, 40, 0, 16],
        ));
        dvd_sup.extend(pgs_segment(
            1000,
            0x14,
            &build_pds(0, &[(0, 16, 128, 128, 0), (1, 235, 128, 128, 255)]),
        ));
        dvd_sup.extend(pgs_segment(1000, 0x15, &o1));
        dvd_sup.extend(pgs_segment(1000, 0x15, &o2));
        dvd_sup.extend(pgs_segment(1000, 0x80, &[]));
        dvd_sup.extend(pgs_segment(2500, 0x16, &build_pcs(0x00, 0, &[])));
        dvd_sup.extend(pgs_segment(2500, 0x80, &[]));
        let dvd_sup_path = dir.path().join("dvd.sup");
        std::fs::write(&dvd_sup_path, dvd_sup).unwrap();

        let dvd_mkv = dir.path().join("dvd.mkv");
        let dvd_status = std::process::Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=size=1280x720:duration=4:rate=1",
                "-i",
            ])
            .arg(&dvd_sup_path)
            .args([
                "-map",
                "0:v",
                "-map",
                "1:s",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-c:s",
                "dvdsub",
                "-y",
            ])
            .arg(&dvd_mkv)
            .status()
            .unwrap();
        assert!(dvd_status.success());

        let dvd_cues = extract_bitmap_subtitles("ffmpeg", "ffprobe", &dvd_mkv, 1, Some(1), None)
            .await
            .unwrap();
        assert_eq!(dvd_cues.len(), 1);
        assert!(dvd_cues[0].width > 0 && dvd_cues[0].width < 1280);
        assert!(dvd_cues[0].height > 0 && dvd_cues[0].height < 720);

        // 3. Reject text-based SRT track with explicit guidance
        let srt_path = dir.path().join("text.srt");
        std::fs::write(&srt_path, "1\n00:00:00,000 --> 00:00:01,000\nHello\n\n").unwrap();
        let text_mkv = dir.path().join("text.mkv");
        let text_status = std::process::Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=size=32x32:duration=1:rate=1",
                "-i",
            ])
            .arg(&srt_path)
            .args([
                "-map",
                "0:v",
                "-map",
                "1:s",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-c:s",
                "srt",
                "-y",
            ])
            .arg(&text_mkv)
            .status()
            .unwrap();
        assert!(text_status.success());

        let text_err = extract_bitmap_subtitles("ffmpeg", "ffprobe", &text_mkv, 1, None, None)
            .await
            .unwrap_err();
        assert!(text_err.to_string().contains("text-based"));

        // 4. Cancellation aborts immediately
        let cancel = CancellationToken::new();
        cancel.cancel();
        let cancel_err =
            extract_bitmap_subtitles("ffmpeg", "ffprobe", &pgs_mkv, 1, None, Some(&cancel))
                .await
                .unwrap_err();
        assert!(cancel_err.to_string().to_lowercase().contains("cancel"));
    }

    #[test]
    fn decode_pgs_rle_handles_short_and_two_byte_rle_runs() {
        // 10x2 raster:
        // Row 0: 00 02 (2x transparent), 00 83 05 (3x color 5), 00 40 02 (2x transparent), 00 C0 03 07 (3x color 7), 00 00 (EOL)
        // Row 1: direct bytes [9, 9], 00 00 (EOL)
        let rle = [
            0x00, 0x02, 0x00, 0x83, 0x05, 0x00, 0x40, 0x02, 0x00, 0xC0, 0x03, 0x07, 0x00, 0x00,
            0x09, 0x09, 0x00, 0x00,
        ];
        let indices = decode_pgs_rle(&rle, 10, 2).unwrap();
        assert_eq!(
            indices,
            vec![
                0, 0, 5, 5, 5, 0, 0, 7, 7, 7, // row 0
                9, 9, 0, 0, 0, 0, 0, 0, 0, 0, // row 1
            ]
        );
    }

    #[test]
    fn parse_ffprobe_packet_probe_extracts_non_16_9_stream_dimensions() {
        let pal_json = br#"{
            "streams": [{"width": 720, "height": 576}],
            "format": {"start_time": "0.000000"},
            "packets": [{"pts_time": "1.200000", "duration_time": "0.800000", "size": "64"}]
        }"#;
        let PacketProbeResult {
            stream_dims: dims,
            events,
        } = parse_ffprobe_packet_probe(pal_json).unwrap();
        assert_eq!(dims, Some((720, 576)));
        assert_eq!(
            events,
            vec![SubtitlePacketEvent {
                pts_ms: 1200,
                duration_ms: Some(800),
            }]
        );

        let oversized_json = br#"{
            "streams": [{"width": 8192, "height": 576}],
            "format": {"start_time": "0.0"},
            "packets": []
        }"#;
        let PacketProbeResult {
            stream_dims: oversized_dims,
            events: empty_events,
        } = parse_ffprobe_packet_probe(oversized_json).unwrap();
        assert_eq!(oversized_dims, None);
        assert!(empty_events.is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn extract_rendered_bitmap_cues_uses_stream_dimensions_and_respects_limit() {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let dir = tempfile::tempdir().unwrap();
        let probe_json = br#"{
            "streams": [{"width": 720, "height": 576}],
            "format": {"start_time": "0.000000"},
            "packets": [
                {"pts_time": "1.000", "duration_time": "0.500", "size": "20"},
                {"pts_time": "2.000", "duration_time": "0.500", "size": "20"},
                {"pts_time": "3.000", "duration_time": "0.500", "size": "20"}
            ]
        }"#;

        // Build a tiny 8x4 PNG with a bright centered 4x2 subtitle block.
        let mut rgba = vec![0u8; 8 * 4 * 4];
        for y in 1..3 {
            for x in 2..6 {
                let off = (y * 8 + x) * 4;
                rgba[off..off + 4].copy_from_slice(&[240, 240, 240, 255]);
            }
        }
        let mut png_bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_bytes, 8, 4);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&rgba).unwrap();
        }
        let png_path = dir.path().join("cue.png");
        std::fs::write(&png_path, &png_bytes).unwrap();
        let probe_path = dir.path().join("probe.json");
        std::fs::write(&probe_path, probe_json).unwrap();
        let args_log = dir.path().join("ffmpeg_args.log");

        let ffprobe_script = dir.path().join("mock-ffprobe");
        let ffmpeg_script = dir.path().join("mock-ffmpeg");
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o700)
            .open(&ffprobe_script)
            .unwrap()
            .write_all(format!("#!/bin/sh\ncat '{}'\n", probe_path.display()).as_bytes())
            .unwrap();
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o700)
            .open(&ffmpeg_script)
            .unwrap()
            .write_all(
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncat '{}'\n",
                    args_log.display(),
                    png_path.display()
                )
                .as_bytes(),
            )
            .unwrap();

        let dummy_video = dir.path().join("movie.mkv");
        std::fs::write(&dummy_video, b"stub").unwrap();

        let cues = extract_rendered_bitmap_cues(
            ffmpeg_script.to_str().unwrap(),
            ffprobe_script.to_str().unwrap(),
            &dummy_video,
            2,
            Some(2),
            None,
        )
        .await
        .unwrap();

        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start_ms, 1000);
        assert_eq!(cues[1].start_ms, 2000);
        let logged = std::fs::read_to_string(&args_log).unwrap();
        assert!(logged.contains("s=720x576:"));
        assert!(logged.contains("[1:v][0:2]overlay=eof_action=pass[v]"));
    }
}
