use super::*;

fn wav_spec(channels: u16, bits: u16, sample_format: hound::SampleFormat) -> hound::WavSpec {
    hound::WavSpec {
        channels,
        sample_rate: 16000,
        bits_per_sample: bits,
        sample_format,
    }
}

#[test]
fn pcm_normalization_preserves_sign_and_channel_average() {
    let directory = tempfile::tempdir().unwrap();
    for bits in [8, 16, 24, 32] {
        for channels in [1, 2, 3, 6] {
            let path = directory.path().join(format!("pcm-{bits}-{channels}.wav"));
            let mut writer =
                hound::WavWriter::create(&path, wav_spec(channels, bits, hound::SampleFormat::Int))
                    .unwrap();
            let half = 1i32 << (bits - 2);
            for sign in [1, -1] {
                for _ in 0..channels {
                    writer.write_sample(sign * half).unwrap();
                }
            }
            for channel in 0..channels {
                writer
                    .write_sample(if channel == 0 { half } else { 0 })
                    .unwrap();
            }
            writer.finalize().unwrap();
            let samples = read_wav_to_f32(&path).unwrap();
            assert_eq!(samples.len(), 3);
            assert_eq!(samples[0], 0.5);
            assert_eq!(samples[1], -0.5);
            assert!((samples[2] - 0.5 / f32::from(channels)).abs() < 1e-7);
        }
    }
}

#[test]
fn float_downmix_and_empty_wav() {
    let directory = tempfile::tempdir().unwrap();
    for channels in [1, 2, 3, 6] {
        let path = directory.path().join(format!("float-{channels}.wav"));
        let mut writer =
            hound::WavWriter::create(&path, wav_spec(channels, 32, hound::SampleFormat::Float))
                .unwrap();
        for value in [0.75f32, -0.25] {
            for _ in 0..channels {
                writer.write_sample(value).unwrap();
            }
        }
        writer.finalize().unwrap();
        assert_eq!(read_wav_to_f32(&path).unwrap(), [0.75, -0.25]);
    }
    let path = directory.path().join("empty.wav");
    hound::WavWriter::create(&path, wav_spec(1, 16, hound::SampleFormat::Int))
        .unwrap()
        .finalize()
        .unwrap();
    assert!(read_wav_to_f32(&path).unwrap().is_empty());
}

#[test]
fn truncated_pcm_and_float_samples_report_errors_instead_of_shortening_audio() {
    let directory = tempfile::tempdir().unwrap();
    for format in [hound::SampleFormat::Int, hound::SampleFormat::Float] {
        let path = directory.path().join("truncated.wav");
        let mut writer = hound::WavWriter::create(&path, wav_spec(2, 32, format)).unwrap();
        for _ in 0..4 {
            match format {
                hound::SampleFormat::Int => writer.write_sample(100i32).unwrap(),
                hound::SampleFormat::Float => writer.write_sample(0.5f32).unwrap(),
            }
        }
        writer.finalize().unwrap();
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(file.metadata().unwrap().len() - 1).unwrap();
        let error = read_wav_to_f32(&path).unwrap_err();
        assert!(
            format!("{error:#}").contains("Invalid WAV sample"),
            "{error:#}"
        );
    }
}

#[test]
fn malformed_headers_missing_files_and_nonfinite_samples_are_errors() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bad.wav");
    assert!(read_wav_to_f32(&path).is_err());
    std::fs::write(&path, b"not a WAV").unwrap();
    assert!(read_wav_to_f32(&path).is_err());
    for sample in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut writer =
            hound::WavWriter::create(&path, wav_spec(1, 32, hound::SampleFormat::Float)).unwrap();
        writer.write_sample(sample).unwrap();
        writer.finalize().unwrap();
        assert!(format!("{:#}", read_wav_to_f32(&path).unwrap_err()).contains("Non-finite"));
    }
}

#[test]
fn incomplete_frames_and_sample_errors_do_not_produce_partial_success() {
    assert!(downmix([Ok(0.5)].into_iter(), 2, 1, 1.0).is_err());
    assert!(
        downmix(
            [Ok(0.5), Err(hound::Error::FormatError("corrupt sample"))].into_iter(),
            2,
            1,
            1.0
        )
        .is_err()
    );
    assert!(downmix([Ok(f32::MAX), Ok(f32::MAX)].into_iter(), 2, 1, 1.0).is_err());
}

#[tokio::test]
async fn stderr_is_drained_but_only_its_tail_is_retained() {
    use tokio::io::AsyncWriteExt;
    let (mut writer, reader) = tokio::io::duplex(1024);
    let write = async {
        writer
            .write_all(&vec![b'x'; STDERR_LIMIT * 3])
            .await
            .unwrap();
        writer.write_all(b"last diagnostic").await.unwrap();
        writer.shutdown().await.unwrap();
    };
    let (tail, ()) = tokio::join!(read_stderr(reader), write);
    let tail = tail.unwrap();
    assert_eq!(tail.len(), STDERR_LIMIT);
    assert!(tail.ends_with(b"last diagnostic"));
}

#[tokio::test]
async fn spawn_failure_and_precancelled_operation_are_distinct() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing-ffmpeg");
    let error = run_audio_command(
        &mut tokio::process::Command::new(&missing),
        "conversion",
        None,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("Failed to spawn"));
    let token = CancellationToken::new();
    token.cancel();
    let error = run_audio_command(
        &mut tokio::process::Command::new(&missing),
        "conversion",
        Some(&token),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("cancelled"));
}

#[tokio::test]
async fn real_ffmpeg_conversion_segmentation_and_failure() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.wav");
    let mut writer =
        hound::WavWriter::create(&source, wav_spec(1, 16, hound::SampleFormat::Int)).unwrap();
    for _ in 0..48000 {
        writer.write_sample(1000i16).unwrap();
    }
    writer.finalize().unwrap();
    let converted = directory.path().join("converted.wav");
    convert_to_wav("ffmpeg", &source, &converted, None)
        .await
        .unwrap();
    assert_eq!(read_wav_to_f32(&converted).unwrap().len(), 48000);
    let segment = directory.path().join("segment.wav");
    extract_wav_segment("ffmpeg", &source, &segment, 1.0, 1.0, None)
        .await
        .unwrap();
    assert_eq!(read_wav_to_f32(&segment).unwrap().len(), 16000);
    for (start, duration) in [
        (f64::NAN, 1.0),
        (-1.0, 1.0),
        (0.0, 0.0),
        (0.0, f64::INFINITY),
    ] {
        assert!(
            extract_wav_segment("missing-ffmpeg", &source, &segment, start, duration, None)
                .await
                .is_err()
        );
    }
    let chunks_dir = directory.path().join("chunks");
    std::fs::create_dir(&chunks_dir).unwrap();
    std::fs::create_dir(chunks_dir.join("chunk_directory.wav")).unwrap();
    let chunks = segment_to_wav_chunks("ffmpeg", &source, &chunks_dir, 1, None)
        .await
        .unwrap();
    assert!(chunks.len() >= 2);
    assert!(chunks.windows(2).all(|pair| pair[0] < pair[1]));
    let count: usize = chunks
        .iter()
        .map(|path| read_wav_to_f32(path).unwrap().len())
        .sum();
    assert_eq!(count, 48000);
    let error = convert_to_wav(
        "ffmpeg",
        &directory.path().join("missing.wav"),
        &converted,
        None,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("FFmpeg audio conversion failed"));
}

#[cfg(unix)]
#[tokio::test]
async fn process_failure_keeps_exit_status_and_stderr_tail_without_deadlock() {
    let mut command = tokio::process::Command::new("sh");
    command.args([
        "-c",
        "head -c 200000 /dev/zero >&2; printf 'final failure' >&2; exit 7",
    ]);
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        run_audio_command(&mut command, "conversion", None),
    )
    .await
    .unwrap()
    .unwrap_err();
    let message = error.to_string();
    assert!(message.contains('7'));
    assert!(message.ends_with("final failure"));
    assert!(message.len() < STDERR_LIMIT + 256);
}

#[cfg(unix)]
async fn wait_for_pid(path: &Path) -> String {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Ok(pid) = tokio::fs::read_to_string(path).await
                && !pid.trim().is_empty()
            {
                return pid.trim().to_owned();
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

#[cfg(unix)]
async fn assert_process_stopped(pid: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let alive = tokio::process::Command::new("kill")
                .args(["-0", pid])
                .stderr(std::process::Stdio::null())
                .status()
                .await
                .unwrap()
                .success();
            if !alive {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Cancelled child must be killed and reaped");
}

#[cfg(unix)]
#[tokio::test]
async fn cancellation_and_dropped_future_stop_the_child() {
    for abort_task in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let pid_path = directory.path().join("child.pid");
        let token = CancellationToken::new();
        let child_token = token.clone();
        let child_pid_path = pid_path.clone();
        let task = tokio::spawn(async move {
            let mut command = tokio::process::Command::new("sh");
            command
                .args(["-c", "echo $$ > \"$1\"; exec sleep 30", "test"])
                .arg(child_pid_path);
            run_audio_command(&mut command, "conversion", Some(&child_token)).await
        });
        let pid = wait_for_pid(&pid_path).await;
        if abort_task {
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        } else {
            token.cancel();
            let error = task.await.unwrap().unwrap_err();
            assert!(error.to_string().contains("cancelled"));
        }
        assert_process_stopped(&pid).await;
    }
}

#[tokio::test]
async fn stderr_read_failures_are_propagated() {
    struct BrokenReader;
    impl tokio::io::AsyncRead for BrokenReader {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            _buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Err(std::io::Error::other("broken diagnostic pipe")))
        }
    }
    assert_eq!(
        read_stderr(BrokenReader).await.unwrap_err().to_string(),
        "broken diagnostic pipe"
    );
}
