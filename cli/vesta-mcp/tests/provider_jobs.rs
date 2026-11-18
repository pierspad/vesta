use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn endpoint(
    body: Value,
) -> (
    String,
    tokio::sync::oneshot::Receiver<()>,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0; 8192];
        loop {
            let count = stream.read(&mut buffer).await.unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&buffer[..count]);
            assert!(bytes.len() < 2 * 1024 * 1024);
            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                let size = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + size {
                    break;
                }
            }
        }
        let _ = started.send(());
        let _ = released.await;
        let body = body.to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes()).await;
    });
    (url, ready, release, task)
}

#[tokio::test]
async fn stdio_provider_jobs_publish_outputs_and_cancel_without_exposing_credentials()
-> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(40), async {
        let input = tempfile::tempdir()?; let output = tempfile::tempdir()?;
        let source = input.path().join("film.srt");
        std::fs::write(&source, "1\n00:00:01,000 --> 00:00:02,000\nHello\n")?;
        let wav = input.path().join("audio.wav");
        let ffmpeg = std::process::Command::new("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "sine=frequency=440:duration=1", "-ar", "16000", "-ac", "1"]).arg(&wav).status()?;
        anyhow::ensure!(ffmpeg.success(), "FFmpeg fixture creation failed");
        let translation_body = json!({"choices":[{"message":{"role":"assistant","content":"[{\"id\":1,\"text\":\"Ciao\"}]"}}]});
        let (translation_url, translation_ready, translation_release, translation_server) = endpoint(translation_body).await;
        let (translation_cancel_url, translation_cancel_ready, translation_cancel_release, translation_cancel_server) = endpoint(json!({"choices":[{"message":{"role":"assistant","content":"[{\"id\":1,\"text\":\"Late\"}]"}}]})).await;
        let (translation_bad_url, translation_bad_ready, translation_bad_release, translation_bad_server) = endpoint(json!({"choices":[{"message":{"role":"assistant","content":"[]"}}]})).await;
        let (speech_url, speech_ready, speech_release, speech_server) = endpoint(json!({"segments":[{"start":0.0,"end":1.0,"text":"Hello speech"}]})).await;
        let (cancel_url, cancel_ready, cancel_release, cancel_server) = endpoint(json!({"segments":[{"start":0.0,"end":1.0,"text":"Late"}]})).await;
        let config = input.path().join("providers.json");
        std::fs::write(&config, json!({
            "translation":{"profiles":{"italian":{"tiers":[[{"provider":"custom","model":"test","api_url":translation_url,"api_key_env":"VESTA_TEST_KEY","rpm":0}]],"batch_size":1},
                "cancel-translation":{"tiers":[[{"provider":"custom","model":"test","api_url":translation_cancel_url,"rpm":0}]],"batch_size":1},
                "bad-translation":{"tiers":[[{"provider":"custom","model":"test","api_url":translation_bad_url,"rpm":0,"max_requests":1}]],"batch_size":1}
            }},
            "transcription":{"profiles":{
                "speech":{"provider":"custom","model":"test","api_url":speech_url,"api_key_env":"VESTA_TEST_KEY"},
                "cancel":{"provider":"custom","model":"test","api_url":cancel_url}
            }}
        }).to_string())?;
        let secret = "mcp-fixture-secret-do-not-return";
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_vesta-mcp"));
        command.arg("--read-root").arg(input.path()).arg("--output-root").arg(output.path()).arg("--provider-config").arg(config).env("VESTA_TEST_KEY", secret);
        let client = ().serve(TokioChildProcess::new(command)?).await?;
        let call = |name: &'static str, args: Value| client.call_tool(CallToolRequestParams::new(name).with_arguments(args.as_object().unwrap().clone()));
        assert_eq!(client.list_all_tools().await?.len(), 12);
        let settings = call("get_configuration", json!({})).await?.structured_content.unwrap();
        assert_eq!(settings["translation_available"], true); assert_eq!(settings["transcription_available"], true);
        assert!(!settings.to_string().contains(secret)); assert!(!settings.to_string().contains("api_url"));
        let bad = call("translate_subtitles", json!({"input":source,"target_language":"it","profile_id":"missing"})).await?;
        assert_eq!(bad.is_error, Some(true)); assert_eq!(std::fs::read_dir(output.path())?.count(), 0);
        let started = call("translate_subtitles", json!({"input":source,"target_language":"it","profile_id":"italian"})).await?.structured_content.unwrap();
        translation_ready.await?;
        let busy = call("transcribe_media", json!({"input":wav,"profile_id":"speech"})).await?;
        assert_eq!(busy.is_error, Some(true));
        translation_release.send(()).unwrap(); translation_server.await?;
        let id = started["job_id"].as_str().unwrap();
        let finished = loop {
            let state = call("get_job", json!({"job_id":id})).await?.structured_content.unwrap();
            if state["status"] != "running" && state["status"] != "cancelling" { break state; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert_eq!(finished["status"], "succeeded", "{finished}");
        assert!(std::fs::read_to_string(finished["result"]["output_path"].as_str().unwrap())?.contains("Ciao"));
        assert!(finished["progress"].is_object()); assert!(!finished.to_string().contains(secret));
        for (profile, ready, release, server, cancel) in [
            ("cancel-translation", translation_cancel_ready, translation_cancel_release, translation_cancel_server, true),
            ("bad-translation", translation_bad_ready, translation_bad_release, translation_bad_server, false),
        ] {
            let started = call("translate_subtitles", json!({"input":source,"profile_id":profile,"target_language":"it"})).await?.structured_content.unwrap();
            ready.await?;
            let id = started["job_id"].as_str().unwrap();
            if cancel { call("cancel_job", json!({"job_id":id})).await?; }
            release.send(()).unwrap(); server.await?;
            let finished = loop {
                let state = call("get_job", json!({"job_id":id})).await?.structured_content.unwrap();
                if state["status"] != "running" && state["status"] != "cancelling" { break state; }
                tokio::time::sleep(Duration::from_millis(10)).await;
            };
            assert_eq!(finished["status"], if cancel { "cancelled" } else { "failed" }, "{finished}");
            assert!(finished["result"].is_null()); assert!(!finished.to_string().contains(secret));
            assert!(!Path::new(started["output_dir"].as_str().unwrap()).join("translated.srt").exists());
        }
        for (profile, ready, release, server, cancel) in [("speech", speech_ready, speech_release, speech_server, false), ("cancel", cancel_ready, cancel_release, cancel_server, true)] {
            let started = call("transcribe_media", json!({"input":wav,"profile_id":profile,"language":"en"})).await?.structured_content.unwrap();
            ready.await?;
            let id = started["job_id"].as_str().unwrap();
            if cancel { call("cancel_job", json!({"job_id":id})).await?; }
            release.send(()).unwrap(); server.await?;
            let finished = loop {
                let state = call("get_job", json!({"job_id":id})).await?.structured_content.unwrap();
                if state["status"] != "running" && state["status"] != "cancelling" { break state; }
                tokio::time::sleep(Duration::from_millis(10)).await;
            };
            assert!(!finished.to_string().contains(secret));
            if cancel {
                assert_eq!(finished["status"], "cancelled", "{finished}");
                assert!(!Path::new(started["output_dir"].as_str().unwrap()).join("transcribed.srt").exists());
            } else {
                assert_eq!(finished["status"], "succeeded", "{finished}");
                let path = Path::new(finished["result"]["output_path"].as_str().unwrap());
                assert!(path.starts_with(output.path())); assert!(std::fs::read_to_string(path)?.contains("Hello speech"));
            }
        }
        client.cancel().await?;
        anyhow::Ok(())
    }).await??;
    Ok(())
}

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

fn create_single_cue_pgs_mkv(dir: &Path) -> std::path::PathBuf {
    let mut sup = Vec::new();
    let mut pds = vec![0u8, 0u8];
    pds.extend_from_slice(&[0, 16, 128, 128, 0, 1, 235, 128, 128, 255]);
    let mut rle = Vec::new();
    for _ in 0..8 {
        rle.extend_from_slice(&[0x00, 0x80 | 20, 1, 0x00, 0x00]);
    }
    let obj_len = (4 + rle.len()) as u32;
    let mut ods = vec![
        0,
        0,
        0,
        0xC0,
        ((obj_len >> 16) & 0xFF) as u8,
        ((obj_len >> 8) & 0xFF) as u8,
        (obj_len & 0xFF) as u8,
    ];
    ods.extend_from_slice(&20u16.to_be_bytes());
    ods.extend_from_slice(&8u16.to_be_bytes());
    ods.extend_from_slice(&rle);

    let mut pcs = Vec::new();
    pcs.extend_from_slice(&1280u16.to_be_bytes());
    pcs.extend_from_slice(&720u16.to_be_bytes());
    pcs.push(0x10);
    pcs.extend_from_slice(&1u16.to_be_bytes());
    pcs.push(0x80);
    pcs.push(0x00);
    pcs.push(0x00);
    pcs.push(1);
    pcs.extend_from_slice(&0u16.to_be_bytes());
    pcs.push(0);
    pcs.push(0);
    pcs.extend_from_slice(&100u16.to_be_bytes());
    pcs.extend_from_slice(&600u16.to_be_bytes());

    sup.extend(pgs_segment(1000, 0x16, &pcs));
    sup.extend(pgs_segment(1000, 0x14, &pds));
    sup.extend(pgs_segment(1000, 0x15, &ods));
    sup.extend(pgs_segment(1000, 0x80, &[]));

    let mut clear_pcs = Vec::new();
    clear_pcs.extend_from_slice(&1280u16.to_be_bytes());
    clear_pcs.extend_from_slice(&720u16.to_be_bytes());
    clear_pcs.push(0x10);
    clear_pcs.extend_from_slice(&10u16.to_be_bytes());
    clear_pcs.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    sup.extend(pgs_segment(2500, 0x16, &clear_pcs));
    sup.extend(pgs_segment(2500, 0x80, &[]));

    let sup_path = dir.join("cue.sup");
    std::fs::write(&sup_path, sup).unwrap();
    let mkv_path = dir.join("cue.mkv");
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-copyts",
            "-f",
            "lavfi",
            "-i",
            "color=size=1280x720:duration=4:rate=1",
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
        .arg(&mkv_path)
        .status()
        .expect("FFmpeg fixture creation failed");
    assert!(status.success());
    mkv_path
}

#[tokio::test]
async fn stdio_ocr_lists_embedded_tracks_and_runs_cancellable_vision_jobs() -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(40), async {
        let input = tempfile::tempdir()?;
        let output = tempfile::tempdir()?;
        let mkv = create_single_cue_pgs_mkv(input.path());

        let (ocr_url, ocr_ready, ocr_release, ocr_server) =
            endpoint(json!({"choices":[{"message":{"role":"assistant","content":"OCR via MCP"}}]}))
                .await;
        let (ocr_cancel_url, ocr_cancel_ready, ocr_cancel_release, ocr_cancel_server) =
            endpoint(json!({"choices":[{"message":{"role":"assistant","content":"Late OCR"}}]}))
                .await;

        let config = input.path().join("providers.json");
        let secret = "mcp-ocr-secret-do-not-return";
        std::fs::write(
            &config,
            json!({
                "translation": {
                    "profiles": {
                        "vision-ocr": {
                            "tiers": [[{
                                "provider": "custom",
                                "model": "vision-test",
                                "api_url": ocr_url,
                                "api_key_env": "VESTA_OCR_KEY",
                                "rpm": 0
                            }]]
                        },
                        "cancel-ocr": {
                            "tiers": [[{
                                "provider": "custom",
                                "model": "vision-test",
                                "api_url": ocr_cancel_url,
                                "rpm": 0
                            }]]
                        }
                    }
                }
            })
            .to_string(),
        )?;

        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_vesta-mcp"));
        command
            .arg("--read-root")
            .arg(input.path())
            .arg("--output-root")
            .arg(output.path())
            .arg("--provider-config")
            .arg(&config)
            .env("VESTA_OCR_KEY", secret);
        let client = ().serve(TokioChildProcess::new(command)?).await?;
        let call = |name: &'static str, args: Value| {
            client.call_tool(
                CallToolRequestParams::new(name).with_arguments(args.as_object().unwrap().clone()),
            )
        };

        let listed = call("list_embedded_subtitles", json!({"input": mkv})).await?;
        assert_ne!(listed.is_error, Some(true));
        let listed_json = listed.structured_content.unwrap();
        let tracks = listed_json["tracks"].as_array().unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0]["codec"], "hdmv_pgs_subtitle");
        assert_eq!(tracks[0]["text_based"], false);
        let track_index = tracks[0]["index"].as_u64().unwrap();

        let bad_lang = call(
            "ocr_subtitles",
            json!({
                "input": mkv,
                "track_index": track_index,
                "profile_id": "vision-ocr",
                "language": "en\ninject"
            }),
        )
        .await?;
        assert_eq!(bad_lang.is_error, Some(true));

        for invalid_args in [
            json!({
                "input": mkv,
                "profile_id": "vision-ocr"
            }),
            json!({
                "input": mkv,
                "track_index": track_index,
                "roi": { "x": 0.08, "y": 0.74, "width": 0.84, "height": 0.20 },
                "profile_id": "vision-ocr"
            }),
            json!({
                "input": mkv,
                "roi": { "x": -0.1, "y": 0.74, "width": 0.84, "height": 0.20 },
                "profile_id": "vision-ocr"
            }),
            json!({
                "input": mkv,
                "roi": { "x": 0.08, "y": 0.74, "width": 0.84, "height": 0.20 },
                "sample_fps": 20.0,
                "profile_id": "vision-ocr"
            }),
        ] {
            let rejected = call("ocr_subtitles", invalid_args).await?;
            assert_eq!(rejected.is_error, Some(true));
        }
        assert_eq!(std::fs::read_dir(output.path())?.count(), 0);

        let started = call(
            "ocr_subtitles",
            json!({
                "input": mkv,
                "track_index": track_index,
                "profile_id": "vision-ocr",
                "language": "en"
            }),
        )
        .await?
        .structured_content
        .unwrap();
        assert_eq!(started["operation"], "ocr");
        ocr_ready.await?;
        ocr_release.send(()).unwrap();
        ocr_server.await?;

        let job_id = started["job_id"].as_str().unwrap();
        let finished = loop {
            let state = call("get_job", json!({"job_id": job_id}))
                .await?
                .structured_content
                .unwrap();
            if state["status"] != "running" && state["status"] != "cancelling" {
                break state;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert_eq!(finished["status"], "succeeded", "{finished}");
        assert_eq!(finished["result"]["subtitle_count"], 1);
        assert!(finished["progress"].is_object());
        assert!(!finished.to_string().contains(secret));
        let out_path = Path::new(finished["result"]["output_path"].as_str().unwrap());
        assert!(out_path.starts_with(output.path()));
        assert!(std::fs::read_to_string(out_path)?.contains("OCR via MCP"));

        let cancel_started = call(
            "ocr_subtitles",
            json!({
                "input": mkv,
                "track_index": track_index,
                "profile_id": "cancel-ocr"
            }),
        )
        .await?
        .structured_content
        .unwrap();
        ocr_cancel_ready.await?;
        let cancel_id = cancel_started["job_id"].as_str().unwrap();
        call("cancel_job", json!({"job_id": cancel_id})).await?;
        ocr_cancel_release.send(()).unwrap();
        ocr_cancel_server.await?;

        let cancelled = loop {
            let state = call("get_job", json!({"job_id": cancel_id}))
                .await?
                .structured_content
                .unwrap();
            if state["status"] != "running" && state["status"] != "cancelling" {
                break state;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert_eq!(cancelled["status"], "cancelled", "{cancelled}");
        assert!(cancelled["result"].is_null());
        assert!(!cancelled.to_string().contains(secret));
        assert!(
            !Path::new(cancel_started["output_dir"].as_str().unwrap())
                .join("ocr.srt")
                .exists()
        );

        client.cancel().await?;
        anyhow::Ok(())
    })
    .await??;
    Ok(())
}

fn create_burned_in_mkv(dir: &Path) -> std::path::PathBuf {
    let mkv_path = dir.join("burned_in.mkv");
    let vf = "drawbox=x=80:y=270:w=220:h=36:color=white@1.0:t=fill:enable='between(t,0.4,1.4)'";
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=640x360:d=2.0:r=10",
            "-vf",
            vf,
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-y",
        ])
        .arg(&mkv_path)
        .status()
        .expect("FFmpeg fixture creation failed");
    assert!(status.success());
    mkv_path
}

#[tokio::test]
async fn stdio_ocr_supports_hardcoded_roi_and_local_ocr_profiles() -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(40), async {
        let input = tempfile::tempdir()?;
        let output = tempfile::tempdir()?;
        let mkv = create_burned_in_mkv(input.path());
        let srt = input.path().join("sample.srt");
        std::fs::write(&srt, "1\n00:00:01,000 --> 00:00:02,000\nHello\n")?;

        let (ocr_url, ocr_ready, ocr_release, ocr_server) = endpoint(
            json!({"choices":[{"message":{"role":"assistant","content":"Hardcoded OCR via MCP"}}]}),
        )
        .await;

        let config = input.path().join("providers.json");
        std::fs::write(
            &config,
            json!({
                "translation": {
                    "profiles": {
                        "vision-roi": {
                            "tiers": [[{
                                "provider": "custom",
                                "model": "vision-test",
                                "api_url": ocr_url,
                                "rpm": 0
                            }]]
                        },
                        "offline-paddle": {
                            "tiers": [[{
                                "provider": "paddle",
                                "model": "paddleocr-v4-mobile-multi"
                            }]]
                        }
                    }
                }
            })
            .to_string(),
        )?;

        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_vesta-mcp"));
        command
            .arg("--read-root")
            .arg(input.path())
            .arg("--output-root")
            .arg(output.path())
            .arg("--provider-config")
            .arg(&config);
        let client = ().serve(TokioChildProcess::new(command)?).await?;
        assert_eq!(client.list_all_tools().await?.len(), 12);

        let call = |name: &'static str, args: Value| {
            client.call_tool(
                CallToolRequestParams::new(name).with_arguments(args.as_object().unwrap().clone()),
            )
        };

        let bad_translate = call(
            "translate_subtitles",
            json!({
                "input": srt,
                "target_language": "it",
                "profile_id": "offline-paddle"
            }),
        )
        .await?;
        assert_eq!(bad_translate.is_error, Some(true));

        let started = call(
            "ocr_subtitles",
            json!({
                "input": mkv,
                "roi": { "x": 0.05, "y": 0.65, "width": 0.90, "height": 0.30 },
                "sample_fps": 5.0,
                "profile_id": "vision-roi",
                "language": "en"
            }),
        )
        .await?
        .structured_content
        .unwrap();
        assert_eq!(started["operation"], "ocr");

        ocr_ready.await?;
        ocr_release.send(()).unwrap();
        ocr_server.await?;

        let job_id = started["job_id"].as_str().unwrap();
        let finished = loop {
            let state = call("get_job", json!({"job_id": job_id}))
                .await?
                .structured_content
                .unwrap();
            if state["status"] != "running" && state["status"] != "cancelling" {
                break state;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert_eq!(finished["status"], "succeeded", "{finished}");
        assert_eq!(finished["result"]["subtitle_count"], 1);
        let out_path = Path::new(finished["result"]["output_path"].as_str().unwrap());
        assert!(out_path.starts_with(output.path()));
        assert!(std::fs::read_to_string(out_path)?.contains("Hardcoded OCR via MCP"));

        client.cancel().await?;
        anyhow::Ok(())
    })
    .await??;
    Ok(())
}
