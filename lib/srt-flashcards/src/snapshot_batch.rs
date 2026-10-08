//! Bounded snapshot extraction. Unknown sources retain individual seek semantics.
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

use crate::SnapshotFormat;
use crate::media::{extract_snapshot, media_command, ms_to_ffmpeg_ts, scale_vf};

const WINDOW_MS: i64 = 8_000;
const MAX_REQUESTS: usize = 8;

#[derive(Clone, Debug)]
pub(crate) struct SnapshotRequest {
    pub seq: usize,
    pub start_ms: i64,
    pub end_ms: i64,
    pub output: PathBuf,
}

impl SnapshotRequest {
    fn midpoint(&self) -> i64 {
        (self.start_ms + (self.end_ms - self.start_ms) / 2).max(0)
    }
}

pub(crate) fn groups(mut requests: Vec<SnapshotRequest>) -> Vec<Vec<SnapshotRequest>> {
    requests.sort_by_key(|r| (r.midpoint(), r.seq));
    let mut groups: Vec<Vec<SnapshotRequest>> = Vec::new();
    for request in requests {
        if groups.last().is_none_or(|group| {
            group.len() >= MAX_REQUESTS || request.midpoint() - group[0].midpoint() > WINDOW_MS
        }) {
            groups.push(Vec::new());
        }
        groups.last_mut().unwrap().push(request);
    }
    groups
}

#[derive(Clone, Debug)]
pub(crate) struct BatchSource {
    timebase_num: i64,
    timebase_den: i64,
    start_us: i64,
}

#[derive(Deserialize)]
struct Probe {
    streams: Vec<Stream>,
    format: Format,
}
#[derive(Deserialize)]
struct Stream {
    #[serde(default)]
    codec_name: String,
    #[serde(default)]
    width: u64,
    #[serde(default)]
    height: u64,
    #[serde(default)]
    time_base: String,
    #[serde(default)]
    r_frame_rate: String,
    #[serde(default)]
    avg_frame_rate: String,
    #[serde(default)]
    disposition: Disposition,
}
#[derive(Default, Deserialize)]
struct Disposition {
    #[serde(default)]
    attached_pic: u8,
}
#[derive(Deserialize)]
struct Format {
    start_time: String,
}

fn ratio(value: &str) -> Option<(i64, i64)> {
    let (num, den) = value.split_once('/')?;
    let (num, den) = (num.parse().ok()?, den.parse().ok()?);
    (num > 0 && den > 0).then_some((num, den))
}

fn micros(value: &str) -> Option<i64> {
    let (seconds, fraction) = value.split_once('.').unwrap_or((value, ""));
    if value.starts_with('-') || fraction.len() > 6 || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    seconds
        .parse::<i64>()
        .ok()?
        .checked_mul(1_000_000)?
        .checked_add(format!("{fraction:0<6}").parse::<i64>().ok()?)
}

impl BatchSource {
    fn from_probe(probe: Probe) -> Option<Self> {
        let stream = probe.streams.first()?;
        let rate = ratio(&stream.r_frame_rate)?;
        let average = ratio(&stream.avg_frame_rate)?;
        // Restrict the default to the measured codecs/resolutions. Attached
        // pictures are harmless only when the baseline auto-selects video 0.
        if !matches!(stream.codec_name.as_str(), "h264" | "hevc")
            || !(1280..=1920).contains(&stream.width)
            || stream.height > 1080
            || stream.disposition.attached_pic != 0
            || i128::from(rate.0) * i128::from(average.1)
                != i128::from(average.0) * i128::from(rate.1)
            || i128::from(rate.0) > i128::from(rate.1) * 240
            || probe.streams.iter().skip(1).any(|other| {
                other.disposition.attached_pic == 0
                    || other.width.saturating_mul(other.height)
                        >= stream.width.saturating_mul(stream.height)
            })
        {
            return None;
        }
        let (timebase_num, timebase_den) = ratio(&stream.time_base)?;
        Some(Self {
            timebase_num,
            timebase_den,
            start_us: micros(&probe.format.start_time)?,
        })
    }

    fn target(&self, midpoint_ms: i64) -> i128 {
        (i128::from(midpoint_ms) * 1000 + i128::from(self.start_us)) * i128::from(self.timebase_den)
    }
    fn pts(&self, pts: i64) -> i128 {
        i128::from(pts) * i128::from(self.timebase_num) * 1_000_000
    }
    fn interval_time(&self, midpoint_ms: i64) -> String {
        let micros = i128::from(midpoint_ms) * 1000 + i128::from(self.start_us);
        format!("{}.{:06}", micros / 1_000_000, micros % 1_000_000)
    }
}

/// Drain both pipes while waiting, and own/reap the direct FFmpeg/FFprobe child.
/// Cancellation does not depend on another media task completing first.
async fn output(
    mut command: tokio::process::Command,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Result<Vec<u8>> {
    if cancel.is_cancelled() {
        anyhow::bail!("Media generation cancelled");
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let mut stdout = child.stdout.take().context("Missing stdout")?;
    let mut stderr = child.stderr.take().context("Missing stderr")?;
    let work = async {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let (a, b, status) = tokio::join!(
            stdout.read_to_end(&mut out),
            stderr.read_to_end(&mut err),
            child.wait()
        );
        a?;
        b?;
        if !status?.success() {
            anyhow::bail!("Media process failed: {}", String::from_utf8_lossy(&err));
        }
        Ok(out)
    };
    tokio::select! {
        _ = cancel.cancelled() => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            anyhow::bail!("Media generation cancelled")
        }
        result = tokio::time::timeout(timeout, work) => match result {
            Ok(result) => result,
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                anyhow::bail!("Snapshot planning/extraction timed out")
            }
        }
    }
}

pub(crate) async fn probe(
    source: &str,
    ffprobe: &str,
    cancel: &CancellationToken,
) -> Option<BatchSource> {
    let mut cmd = media_command(ffprobe);
    cmd.args([
        "-v", "error", "-select_streams", "v", "-show_entries",
        "stream=codec_name,width,height,time_base,r_frame_rate,avg_frame_rate:stream_disposition=attached_pic:format=start_time",
        "-of", "json", source,
    ]);
    let bytes = output(cmd, cancel, Duration::from_secs(10)).await.ok()?;
    BatchSource::from_probe(serde_json::from_slice(&bytes).ok()?)
}

#[derive(Debug, Deserialize)]
struct Packet {
    pts: Option<i64>,
    dts: Option<i64>,
    #[serde(default)]
    flags: String,
}
#[derive(Deserialize)]
struct Packets {
    packets: Vec<Packet>,
}

fn plan(
    source: &BatchSource,
    requests: &[SnapshotRequest],
    packets: &[Packet],
) -> Vec<Option<i64>> {
    // Missing timestamp data and excessive probe output are never guessed.
    if packets.is_empty() || packets.len() > 20_000 || packets.iter().any(|p| p.pts.is_none()) {
        return vec![None; requests.len()];
    }
    let pts: BTreeSet<i64> = packets.iter().filter_map(|p| p.pts).collect();
    let keys: Vec<_> = packets.iter().filter(|p| p.flags.contains('K')).collect();
    if keys.is_empty() {
        return vec![None; requests.len()];
    }
    let first = source.target(requests[0].midpoint());
    let last = source.target(requests.last().unwrap().midpoint());
    let interior: Vec<_> = pts
        .iter()
        .copied()
        .filter(|&p| first <= source.pts(p) && source.pts(p) <= last)
        .collect();
    let deltas: Vec<_> = interior
        .windows(2)
        .map(|p| i128::from(p[1]) - i128::from(p[0]))
        .collect();
    if let (Some(min), Some(max)) = (deltas.iter().min(), deltas.iter().max())
        && max - min > 1
    {
        return vec![None; requests.len()];
    }
    requests
        .iter()
        .map(|request| {
            let target = source.target(request.midpoint());
            if keys.iter().any(|packet| {
                // Past keyframes cannot change this seek even with omitted DTS.
                // Future keyframes with unknown DTS remain unsafe.
                target < source.pts(packet.pts.unwrap())
                    && packet.dts.is_none_or(|dts| source.pts(dts) <= target)
            }) {
                return None;
            }
            pts.iter().copied().find(|&pts| source.pts(pts) >= target)
        })
        .collect()
}

#[derive(Clone)]
pub(crate) struct SnapshotSettings {
    pub source: String,
    pub ffmpeg: String,
    pub ffprobe: String,
    pub width: u32,
    pub height: u32,
    pub crop: u32,
    pub format: SnapshotFormat,
    pub quality: u8,
    pub batch_source: Option<BatchSource>,
    pub cancel: CancellationToken,
}

impl SnapshotSettings {
    async fn batch(&self, requests: &[SnapshotRequest]) -> Result<Vec<bool>> {
        let source = self
            .batch_source
            .as_ref()
            .context("Unsupported snapshot source")?;
        let first = requests[0].midpoint();
        let last = requests.last().unwrap().midpoint();
        let interval = format!(
            "{}%{}",
            source.interval_time(first),
            source.interval_time(last + 1000)
        );
        let mut probe = media_command(&self.ffprobe);
        probe.args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-read_intervals",
            &interval,
            "-show_packets",
            "-show_entries",
            "packet=pts,dts,flags",
            "-of",
            "json",
            &self.source,
        ]);
        let bytes = output(probe, &self.cancel, Duration::from_secs(10)).await?;
        anyhow::ensure!(bytes.len() <= 4 * 1024 * 1024, "Oversized snapshot probe");
        let packets: Packets = serde_json::from_slice(&bytes)?;
        let plan = plan(source, requests, &packets.packets);
        let selected: BTreeSet<_> = plan.iter().flatten().copied().collect();
        if selected.is_empty() {
            return Ok(vec![false; requests.len()]);
        }
        // Keep temporary images alive until every selected image is checked.
        let temp = tempfile::tempdir()?;
        let pattern = temp.path().join("%03d.webp");
        let expression = selected
            .iter()
            .map(|pts| format!("eq(pts,{pts})"))
            .collect::<Vec<_>>()
            .join("+");
        let filter = format!(
            "select='{expression}',{}",
            scale_vf(self.width, self.height, self.crop)
        );
        let extraction = |modern_sync: bool| {
            let mut cmd = media_command(&self.ffmpeg);
            cmd.args([
                "-nostdin",
                "-loglevel",
                "error",
                "-y",
                "-copyts",
                "-ss",
                &ms_to_ffmpeg_ts(first),
                "-i",
                &self.source,
                "-an",
                "-sn",
                "-dn",
                "-vframes",
                &selected.len().to_string(),
                "-vf",
                &filter,
            ]);
            cmd.args(self.format.ffmpeg_args(self.quality));
            if modern_sync {
                cmd.args(["-fps_mode", "passthrough"]);
            } else {
                cmd.args(["-vsync", "0"]);
            }
            cmd.arg(&pattern);
            cmd
        };
        let result = output(extraction(true), &self.cancel, Duration::from_secs(30)).await;
        match result {
            Err(error) if error.to_string().contains("Unrecognized option 'fps_mode'") => {
                // FFmpeg 4.x uses -vsync; recent versions removed that alias.
                // Option parsing fails before decoding or creating output files.
                output(extraction(false), &self.cancel, Duration::from_secs(30)).await?;
            }
            result => {
                result?;
            }
        }
        let selected: Vec<_> = selected.into_iter().collect();
        for n in 1..=selected.len() {
            anyhow::ensure!(
                std::fs::metadata(temp.path().join(format!("{n:03}.webp")))?.len() > 0,
                "Incomplete snapshot batch"
            );
        }
        let mut copied = vec![false; requests.len()];
        for (i, request) in requests.iter().enumerate() {
            if self.cancel.is_cancelled() {
                anyhow::bail!("Media generation cancelled");
            }
            if let Some(pts) = plan[i] {
                let n = selected.binary_search(&pts).unwrap() + 1;
                std::fs::copy(temp.path().join(format!("{n:03}.webp")), &request.output)?;
                copied[i] = true;
            }
        }
        Ok(copied)
    }

    pub async fn extract(
        &self,
        requests: Vec<SnapshotRequest>,
    ) -> Vec<(&'static str, Result<()>, usize)> {
        let done = if requests.len() > 1 && self.batch_source.is_some() {
            match self.batch(&requests).await {
                Ok(done) => done,
                Err(error) => {
                    if !self.cancel.is_cancelled() {
                        eprintln!(
                            "Snapshot batching unavailable; using individual extraction: {error}"
                        );
                    }
                    vec![false; requests.len()]
                }
            }
        } else {
            vec![false; requests.len()]
        };
        let mut results = Vec::with_capacity(requests.len());
        for (request, done) in requests.into_iter().zip(done) {
            let result = if self.cancel.is_cancelled() {
                Err(anyhow::anyhow!("Media generation cancelled"))
            } else if done {
                Ok(())
            } else {
                extract_snapshot(
                    &self.source,
                    &request.output,
                    request.start_ms,
                    request.end_ms,
                    self.width,
                    self.height,
                    self.crop,
                    self.format,
                    self.quality,
                    &self.ffmpeg,
                )
                .await
            };
            results.push(("snapshot", result, request.seq));
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(seq: usize, midpoint: i64, directory: &std::path::Path) -> SnapshotRequest {
        SnapshotRequest {
            seq,
            start_ms: midpoint - 10,
            end_ms: midpoint + 10,
            output: directory.join(format!("{seq}.webp")),
        }
    }

    fn source() -> BatchSource {
        BatchSource {
            timebase_num: 1,
            timebase_den: 24_000,
            start_us: 0,
        }
    }

    #[test]
    fn unverified_resolutions_stream_selection_and_rates_use_individual_extraction() {
        let valid = serde_json::json!({"streams":[{"codec_name":"h264", "width":1920, "height":1080,
            "time_base":"1/24000", "r_frame_rate":"24000/1001", "avg_frame_rate":"24000/1001"}],
            "format":{"start_time":"0.000000"}});
        assert!(BatchSource::from_probe(serde_json::from_value(valid.clone()).unwrap()).is_some());
        for (key, value) in [
            ("width", serde_json::json!(3840)),
            ("avg_frame_rate", serde_json::json!("0/0")),
            ("codec_name", serde_json::json!("vp9")),
        ] {
            let mut probe = valid.clone();
            probe["streams"][0][key] = value;
            assert!(BatchSource::from_probe(serde_json::from_value(probe).unwrap()).is_none());
        }
        let mut ambiguous = valid.clone();
        ambiguous["streams"]
            .as_array_mut()
            .unwrap()
            .push(valid["streams"][0].clone());
        assert!(BatchSource::from_probe(serde_json::from_value(ambiguous).unwrap()).is_none());
    }

    #[test]
    fn grouping_bounds_span_and_size_and_keeps_card_identity() {
        let requests = (1..=20)
            .rev()
            .map(|seq| request(seq, seq as i64 * 1000, std::path::Path::new(".")))
            .collect();
        let groups = groups(requests);
        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), [8, 8, 4]);
        assert!(
            groups
                .iter()
                .all(|g| g.last().unwrap().midpoint() - g[0].midpoint() <= WINDOW_MS)
        );
        assert_eq!(
            groups
                .into_iter()
                .flatten()
                .map(|r| r.seq)
                .collect::<Vec<_>>(),
            (1..=20).collect::<Vec<_>>()
        );
    }

    #[test]
    fn reordered_keyframe_boundary_keeps_individual_seek() {
        let requests = vec![
            request(1, 90561, std::path::Path::new(".")),
            request(2, 90650, std::path::Path::new(".")),
        ];
        let packets = vec![
            Packet {
                pts: Some(2_175_173),
                dts: Some(2_173_171),
                flags: "K__".into(),
            },
            Packet {
                pts: Some(2_176_174),
                dts: Some(2_174_172),
                flags: "___".into(),
            },
            Packet {
                pts: Some(2_174_172),
                dts: Some(2_175_173),
                flags: "___".into(),
            },
        ];
        // First request lies between DTS 90.5487 and PTS 90.6321.
        assert_eq!(
            plan(&source(), &requests, &packets),
            [None, Some(2_176_174)]
        );
    }

    #[test]
    fn missing_dts_of_past_keyframe_does_not_disable_safe_selection() {
        let requests = vec![
            request(1, 100, std::path::Path::new(".")),
            request(2, 150, std::path::Path::new(".")),
        ];
        let mut packets = (0..=5)
            .map(|i| Packet {
                pts: Some(i * 1001),
                dts: Some(i * 1001),
                flags: String::new(),
            })
            .collect::<Vec<_>>();
        packets[0].flags = "K".into();
        packets[0].dts = None;
        assert_eq!(
            plan(&source(), &requests, &packets),
            [Some(3003), Some(4004)]
        );
        packets[5].flags = "K".into();
        packets[5].dts = None;
        assert_eq!(plan(&source(), &requests, &packets), [None, None]);
    }

    #[test]
    fn unknown_timestamps_and_irregular_packet_spacing_are_not_guessed() {
        let requests = vec![
            request(1, 0, std::path::Path::new(".")),
            request(2, 200, std::path::Path::new(".")),
        ];
        let packets = vec![
            Packet {
                pts: Some(0),
                dts: Some(0),
                flags: "K".into(),
            },
            Packet {
                pts: Some(1001),
                dts: Some(1001),
                flags: String::new(),
            },
            Packet {
                pts: Some(3003),
                dts: Some(3003),
                flags: String::new(),
            },
        ];
        assert_eq!(plan(&source(), &requests, &packets), [None, None]);
        assert_eq!(
            plan(
                &source(),
                &requests,
                &[Packet {
                    pts: None,
                    dts: None,
                    flags: "K".into()
                }]
            ),
            [None, None]
        );
        assert!(ratio("0/0").is_none());
        assert!(micros("N/A").is_none());
    }

    async fn fixture(folder: &std::path::Path, vfr: bool, offset: bool) -> PathBuf {
        let path = folder.join("source.mp4");
        let mut command = media_command("ffmpeg");
        command.args([
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1280x720:rate=24:duration=6",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-g",
            "48",
            "-keyint_min",
            "48",
            "-sc_threshold",
            "0",
            "-bf",
            "2",
        ]);
        if vfr {
            let help = media_command("ffmpeg")
                .args(["-hide_banner", "-h", "full"])
                .output()
                .await
                .unwrap();
            let modern = String::from_utf8_lossy(&help.stdout).contains("-fps_mode");
            command.args(["-vf", "select='not(eq(mod(n,5),0))'"]);
            if modern {
                command.args(["-fps_mode", "vfr"]);
            } else {
                command.args(["-vsync", "2"]);
            }
        }
        if offset {
            command.args(["-output_ts_offset", "5"]);
        }
        command.arg("-y").arg(&path);
        let out = command.output().await.unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        path
    }

    // Run sequentially in one integration test to avoid competing media work.
    #[tokio::test]
    async fn real_ffmpeg_preserves_frames_crop_offset_duplicates_and_vfr_fallback() {
        for (vfr, offset) in [(false, false), (false, true), (true, false)] {
            let directory = tempfile::tempdir().unwrap();
            let input = fixture(directory.path(), vfr, offset).await;
            let cancel = CancellationToken::new();
            let batch_source = probe(input.to_str().unwrap(), "ffprobe", &cancel).await;
            assert_eq!(batch_source.is_none(), vfr);
            let baseline_dir = directory.path().join("baseline");
            let batch_dir = directory.path().join("batch");
            std::fs::create_dir(&baseline_dir).unwrap();
            std::fs::create_dir(&batch_dir).unwrap();
            let mids = [501, 502, 1917, 1950, 1999, 2000, 2042, 3950, 4000];
            for (i, mid) in mids.iter().copied().enumerate() {
                extract_snapshot(
                    input.to_str().unwrap(),
                    &baseline_dir.join(format!("{}.webp", i + 1)),
                    mid - 10,
                    mid + 10,
                    160,
                    120,
                    4,
                    SnapshotFormat::Webp,
                    80,
                    "ffmpeg",
                )
                .await
                .unwrap();
            }
            let settings = SnapshotSettings {
                source: input.to_string_lossy().into(),
                ffmpeg: "ffmpeg".into(),
                ffprobe: "ffprobe".into(),
                width: 160,
                height: 120,
                crop: 4,
                format: SnapshotFormat::Webp,
                quality: 80,
                batch_source,
                cancel,
            };
            let requests = mids
                .iter()
                .enumerate()
                .map(|(i, &mid)| request(i + 1, mid, &batch_dir))
                .collect();
            for group in groups(requests) {
                if !vfr && group.len() > 1 {
                    let copied = settings.batch(&group).await.unwrap();
                    assert!(
                        copied.iter().any(|copied| *copied),
                        "Batch must actually extract frames"
                    );
                }
                for (_, result, _) in settings.extract(group).await {
                    result.unwrap();
                }
            }
            for i in 1..=mids.len() {
                assert_eq!(
                    std::fs::read(baseline_dir.join(format!("{i}.webp"))).unwrap(),
                    std::fs::read(batch_dir.join(format!("{i}.webp"))).unwrap(),
                    "Frame {i}: VFR={vfr}, offset={offset}"
                );
            }
        }
    }

    #[tokio::test]
    async fn native_generation_keeps_per_card_progress_and_identical_apkg_images() {
        use std::collections::BTreeMap;
        use std::io::Read;
        use std::sync::Mutex;
        let directory = tempfile::tempdir().unwrap();
        let input = fixture(directory.path(), false, false).await;
        let srt = directory.path().join("source.srt");
        std::fs::write(
            &srt,
            (0..5)
                .map(|i| {
                    format!(
                        "{}\n00:00:{i:02},000 --> 00:00:{:02},000\nLine {}.\n\n",
                        i + 1,
                        i + 1,
                        i + 1
                    )
                })
                .collect::<String>(),
        )
        .unwrap();
        let mut reference = None;
        for optimize in [false, true] {
            let progress = Mutex::new(Vec::new());
            let config = crate::FlashcardConfig {
                target_subs_path: srt.to_string_lossy().into(),
                video_path: Some(input.to_string_lossy().into()),
                output_dir: directory
                    .path()
                    .join(if optimize { "batch" } else { "baseline" })
                    .to_string_lossy()
                    .into(),
                deck_name: "Progress".into(),
                generate_snapshots: true,
                snapshot_width: 160,
                snapshot_height: 120,
                crop_bottom: 4,
                cpu_cores: Some(2),
                optimize_video: optimize,
                export_format: Some("apkg".into()),
                ..crate::FlashcardConfig::default()
            };
            let result = crate::generate(
                config,
                crate::MediaTools::default(),
                CancellationToken::new(),
                &|event| {
                    if event.message == "flashcards.progress.extractingMedia" {
                        progress.lock().unwrap().push((event.current, event.total));
                    }
                },
            )
            .await
            .unwrap();
            assert!(result.success);
            assert_eq!(result.snapshots, 5);
            assert_eq!(result.cards_generated, 5);
            assert_eq!(
                *progress.lock().unwrap(),
                (1..=5).map(|i| (i, 5)).collect::<Vec<_>>()
            );
            let mut archive =
                zip::ZipArchive::new(std::fs::File::open(result.apkg_path.unwrap()).unwrap())
                    .unwrap();
            let manifest: BTreeMap<String, String> =
                serde_json::from_reader(archive.by_name("media").unwrap()).unwrap();
            assert_eq!(manifest.len(), 5);
            let mut bytes = BTreeMap::new();
            for (entry, name) in manifest {
                let mut contents = Vec::new();
                archive
                    .by_name(&entry)
                    .unwrap()
                    .read_to_end(&mut contents)
                    .unwrap();
                assert!(!contents.is_empty());
                bytes.insert(name, contents);
            }
            if let Some(ref baseline) = reference {
                assert_eq!(&bytes, baseline);
            } else {
                reference = Some(bytes);
            }
        }
    }

    #[tokio::test]
    async fn failed_batch_retries_each_snapshot_with_original_parameters() {
        let directory = tempfile::tempdir().unwrap();
        let input = fixture(directory.path(), false, false).await;
        let settings = SnapshotSettings {
            source: input.to_string_lossy().into(),
            ffmpeg: "ffmpeg".into(),
            ffprobe: directory
                .path()
                .join("missing-ffprobe")
                .to_string_lossy()
                .into(),
            width: 160,
            height: 120,
            crop: 0,
            format: SnapshotFormat::Webp,
            quality: 80,
            batch_source: Some(source()),
            cancel: CancellationToken::new(),
        };
        for (_, result, _) in settings
            .extract(vec![
                request(1, 500, directory.path()),
                request(2, 1500, directory.path()),
            ])
            .await
        {
            result.unwrap();
        }
        assert!(directory.path().join("1.webp").metadata().unwrap().len() > 0);
        assert!(directory.path().join("2.webp").metadata().unwrap().len() > 0);
    }
}
