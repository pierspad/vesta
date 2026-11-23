use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

#[tokio::test]
async fn stdio_client_discovers_tools_previews_and_generates_a_deck() -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(30), async {
        let input = tempfile::tempdir()?;
        let output = tempfile::tempdir()?;
        let source = input.path().join("film.srt");
        std::fs::write(&source, "1\n00:00:01,000 --> 00:00:02,000\nHello there\n\n2\n00:00:03,000 --> 00:00:04,000\nSecond line\n")?;
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_vesta-mcp"));
        command.arg("--read-root").arg(input.path()).arg("--output-root").arg(output.path())
            .env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY");
        let client = ().serve(TokioChildProcess::new(command)?).await?;
        let tools = client.list_all_tools().await?;
        assert_eq!(tools.len(), 18);
        for tool in &tools { assert!(tool.input_schema.contains_key("type")); }
        for expected in [
            "extract_embedded_subtitle",
            "sync_subtitles",
            "autosync_subtitles",
            "refine_flashcards",
            "export_dueling_subtitles",
            "merge_apkg",
        ] {
            assert!(
                tools.iter().any(|t| t.name == expected),
                "missing expected MCP tool {expected}"
            );
        }
        let call = |name: &'static str, args: Value| {
            client.call_tool(CallToolRequestParams::new(name).with_arguments(args.as_object().unwrap().clone()))
        };
        let plugins_res = call("list_plugins", json!({})).await?;
        assert_ne!(plugins_res.is_error, Some(true));
        let configuration = call("get_configuration", json!({})).await?;
        let settings = configuration.structured_content.unwrap();
        assert_eq!(settings["max_active_generations"], 1);
        assert_eq!(settings["translation_available"], false);
        assert_eq!(settings["transcription_available"], false);
        for name in ["translate_subtitles", "transcribe_media", "ocr_subtitles"] {
            let args = match name {
                "translate_subtitles" => {
                    json!({"input": source, "profile_id": "missing", "target_language": "it"})
                }
                "ocr_subtitles" => {
                    json!({"input": source, "track_index": 1, "profile_id": "missing"})
                }
                _ => json!({"input": source, "profile_id": "missing"}),
            };
            let rejected = call(name, args).await?;
            assert_eq!(rejected.is_error, Some(true));
        }
        assert_eq!(std::fs::read_dir(output.path())?.count(), 0);
        let info = call("inspect_subtitles", json!({"path": source})).await?;
        assert_ne!(info.is_error, Some(true));
        assert_eq!(info.structured_content.as_ref().unwrap()["count"], 2);
        let preview = call("preview_flashcards", json!({"target": source, "offset":1, "limit":1})).await?;
        let preview = preview.structured_content.unwrap();
        assert_eq!(preview["total"], 2);
        assert_eq!(preview["rows"].as_array().unwrap().len(), 1);
        assert_eq!(preview["rows"][0]["subs1_text"], "Second line");
        assert_eq!(std::fs::read_dir(output.path())?.count(), 0, "preview must not write outputs");
        let bad = call("preview_flashcards", json!({"target": source, "limit":0})).await?;
        assert_eq!(bad.is_error, Some(true));
        let outside = tempfile::tempdir()?;
        let forbidden = outside.path().join("secret.srt");
        std::fs::write(&forbidden, "private")?;
        assert_eq!(call("inspect_subtitles", json!({"path":forbidden})).await?.is_error, Some(true));
        assert_eq!(call("list_embedded_subtitles", json!({"input":forbidden})).await?.is_error, Some(true));
        let invalid = call("generate_flashcards", json!({"target": source,"deck":"Invalid","format":"apkg","include_audio":true})).await?;
        assert_eq!(invalid.is_error, Some(true));
        assert_eq!(std::fs::read_dir(output.path())?.count(), 0);
        for format in ["tsv", "apkg"] {
        let job = call("generate_flashcards", json!({"target": source,"deck":"MCP Test","format":format})).await?;
        assert_ne!(job.is_error, Some(true));
        let job = job.structured_content.unwrap();
        let id = job["job_id"].as_str().unwrap();
        let completed = loop {
            let response = call("get_job", json!({"job_id":id})).await?;
            let snapshot = response.structured_content.unwrap();
            match snapshot["status"].as_str().unwrap() {
                "running" | "cancelling" => tokio::time::sleep(Duration::from_millis(10)).await,
                _ => break snapshot,
            }
        };
        assert_eq!(completed["status"], "succeeded", "{completed}");
        assert_eq!(completed["result"]["cards_generated"], 2);
        let path_key = if format == "tsv" { "tsv_path" } else { "apkg_path" };
        let exported = Path::new(completed["result"][path_key].as_str().unwrap());
        assert!(exported.starts_with(output.path()));
        if format == "tsv" { assert!(std::fs::read_to_string(exported)?.contains("Hello there")); }
        else { assert!(std::fs::read(exported)?.starts_with(b"PK")); }
        // Cancelling a finished job does not rewrite its terminal status.
        assert_eq!(call("cancel_job", json!({"job_id":id})).await?.structured_content.unwrap()["status"], "succeeded");
        }
        assert_eq!(std::fs::read_dir(output.path())?.count(), 2);
        assert_eq!(call("get_job", json!({"job_id":"missing"})).await?.is_error, Some(true));

        // ── sync_subtitles ──────────────────────────────────────────────
        let before_sync = std::fs::read_dir(output.path())?.count();
        for bad_sync in [
            json!({"input": source}),
            json!({"input": source, "offset_ms": 500, "anchors": [{"subtitle_id": 1, "corrected_time_ms": 1500}]}),
            json!({"input": source, "speed": 5.0}),
            json!({"input": source, "anchors": [{"subtitle_id": 99, "corrected_time_ms": 1500}]}),
            json!({"input": forbidden, "offset_ms": 500}),
        ] {
            assert_eq!(call("sync_subtitles", bad_sync).await?.is_error, Some(true));
        }
        assert_eq!(std::fs::read_dir(output.path())?.count(), before_sync);

        let offset_sync = call("sync_subtitles", json!({"input": source, "offset_ms": 1500}))
            .await?
            .structured_content
            .unwrap();
        let offset_path = Path::new(offset_sync["output_path"].as_str().unwrap());
        assert!(offset_path.starts_with(output.path()));
        let offset_text = std::fs::read_to_string(offset_path)?;
        assert!(offset_text.contains("00:00:02,500 --> 00:00:03,500"));
        assert!(offset_text.contains("00:00:04,500 --> 00:00:05,500"));

        let anchor_sync = call(
            "sync_subtitles",
            json!({
                "input": source,
                "anchors": [
                    {"subtitle_id": 1, "corrected_time_ms": 2000},
                    {"subtitle_id": 2, "corrected_time_ms": 6000}
                ]
            }),
        )
        .await?
        .structured_content
        .unwrap();
        assert_eq!(anchor_sync["anchor_count"], 2);
        let anchor_text = std::fs::read_to_string(anchor_sync["output_path"].as_str().unwrap())?;
        assert!(anchor_text.contains("00:00:02,000 --> 00:00:04,000"));
        assert!(anchor_text.contains("00:00:06,000 --> 00:00:07,000"));

        // ── export_dueling_subtitles ────────────────────────────────────
        let native = input.path().join("native.srt");
        std::fs::write(&native, "1\n00:00:01,100 --> 00:00:01,900\nCiao!\n")?;
        let dueling_ass = call(
            "export_dueling_subtitles",
            json!({"target": source, "native": native, "format": "ass"}),
        )
        .await?
        .structured_content
        .unwrap();
        let ass_content = std::fs::read_to_string(dueling_ass["output_path"].as_str().unwrap())?;
        assert!(ass_content.contains("[V4+ Styles]"));
        assert!(ass_content.contains("Hello there"));
        assert!(ass_content.contains("Ciao!"));

        let dueling_srt = call(
            "export_dueling_subtitles",
            json!({
                "target": source,
                "native": native,
                "format": "srt",
                "target_first": false,
                "include_unmatched": false
            }),
        )
        .await?
        .structured_content
        .unwrap();
        assert_eq!(dueling_srt["line_count"], 1);
        let srt_content = std::fs::read_to_string(dueling_srt["output_path"].as_str().unwrap())?;
        assert!(srt_content.contains("Ciao!\nHello there"));

        // ── merge_apkg (including chaining from output_root) ───────────
        let ep1_src = input.path().join("ep1.srt");
        let ep2_src = input.path().join("ep2.srt");
        std::fs::write(&ep1_src, "1\n00:00:01,000 --> 00:00:02,000\nEpisode one\n")?;
        std::fs::write(&ep2_src, "1\n00:00:01,000 --> 00:00:02,000\nEpisode two\n")?;
        let mut apkg_paths = Vec::new();
        for (deck_name, src) in [("Ep1", &ep1_src), ("Ep2", &ep2_src)] {
            let job = call(
                "generate_flashcards",
                json!({"target": src, "deck": deck_name, "format": "apkg"}),
            )
            .await?
            .structured_content
            .unwrap();
            let jid = job["job_id"].as_str().unwrap();
            let done = loop {
                let state = call("get_job", json!({"job_id": jid})).await?.structured_content.unwrap();
                if state["status"] != "running" && state["status"] != "cancelling" {
                    break state;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            };
            assert_eq!(done["status"], "succeeded", "{done}");
            apkg_paths.push(done["result"]["apkg_path"].as_str().unwrap().to_string());
        }
        assert_eq!(call("merge_apkg", json!({"inputs": [&apkg_paths[0]]})).await?.is_error, Some(true));
        assert_eq!(call("merge_apkg", json!({"inputs": [source, &apkg_paths[1]]})).await?.is_error, Some(true));
        let merged = call("merge_apkg", json!({"inputs": apkg_paths}))
            .await?
            .structured_content
            .unwrap();
        let merged_path = Path::new(merged["output_path"].as_str().unwrap());
        assert!(merged_path.starts_with(output.path()));
        assert_eq!(merged["input_count"], 2);
        assert!(std::fs::read(merged_path)?.starts_with(b"PK"));

        client.cancel().await?;
        anyhow::Ok(())
    }).await??;
    Ok(())
}

#[test]
fn help_explains_directory_boundaries() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vesta-mcp"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--read-root"));
    assert!(help.contains("--output-root"));
    assert!(help.contains("Both roots must already exist"));
    assert!(help.contains("not an interactive prompt"));
    assert!(help.contains("vesta-cli <operation> --help"));
}
