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
        assert_eq!(tools.len(), 6);
        for tool in &tools { assert!(tool.input_schema.contains_key("type")); }
        let call = |name: &'static str, args: Value| {
            client.call_tool(CallToolRequestParams::new(name).with_arguments(args.as_object().unwrap().clone()))
        };
        let configuration = call("get_configuration", json!({})).await?;
        assert_eq!(configuration.structured_content.unwrap()["max_active_generations"], 1);
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
