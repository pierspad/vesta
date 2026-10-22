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
        assert_eq!(client.list_all_tools().await?.len(), 8);
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
