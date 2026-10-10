use srt_parser::{Subtitle, Timestamp};
use srt_translate::{ApiType, PoolEntry, Translator, TranslatorConfig, TranslatorPool};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

fn source() -> HashMap<u32, Subtitle> {
    HashMap::from([(
        1,
        Subtitle {
            id: 1,
            start: Timestamp { milliseconds: 1000 },
            end: Timestamp { milliseconds: 2000 },
            text: "Hello".into(),
        },
    )])
}
fn translator(url: String) -> Translator {
    Translator::new(TranslatorConfig {
        api_type: ApiType::Local,
        api_key: None,
        base_url: url,
        model: "test".into(),
    })
}
fn pool(url: String) -> TranslatorPool {
    vec![vec![PoolEntry {
        translator: translator(url),
        rate_limiter: None,
        max_requests: None,
        label: "test".into(),
    }]]
}

async fn server() -> (
    String,
    tokio::sync::oneshot::Receiver<()>,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    server_with_id(1).await
}

async fn server_with_id(
    id: u32,
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
        let mut bytes = [0u8; 8192];
        assert!(stream.read(&mut bytes).await.unwrap() > 0);
        let _ = started.send(());
        let _ = released.await;
        let body =
            serde_json::json!({"choices":[{"message":{"role":"assistant","content":serde_json::json!([{ "id":id, "text":"Ciao" }]).to_string()}}]}).to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        // Cancellation may already have closed the connection.
        let _ = stream.write_all(response.as_bytes()).await;
    });
    (url, ready, release, task)
}

async fn tiered(
    pool: TranslatorPool,
    path: &Path,
    token: CancellationToken,
    size: usize,
) -> anyhow::Result<HashMap<u32, Subtitle>> {
    srt_translate::translate_subtitles_tiered_cancellable(
        pool,
        source(),
        "it",
        size,
        0,
        None,
        path,
        |_| {},
        token,
    )
    .await
}

#[tokio::test]
async fn zero_batch_and_empty_endpoint_or_limiter_lists_are_errors() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("output.srt");
    let url = "http://127.0.0.1:1/v1";
    assert!(
        tiered(pool(url.into()), &path, CancellationToken::new(), 0)
            .await
            .unwrap_err()
            .to_string()
            .contains("batch size")
    );
    for (translators, limiters) in [
        (Vec::new(), None),
        (vec![translator(url.into())], Some(Vec::new())),
    ] {
        assert!(
            srt_translate::translate_subtitles_with_rate_limit_cancellable(
                translators,
                limiters,
                source(),
                "it",
                1,
                0,
                None,
                &path,
                |_| {},
                CancellationToken::new(),
            )
            .await
            .is_err()
        );
    }
    assert!(!path.exists());
}

#[tokio::test]
async fn legacy_and_tiered_apis_use_the_same_translation_and_saved_output() {
    let directory = tempfile::tempdir().unwrap();
    let mut outputs = Vec::new();
    for legacy in [false, true] {
        let path = directory.path().join(format!("{legacy}.srt"));
        let (url, _, release, server) = server().await;
        release.send(()).unwrap();
        let result = if legacy {
            srt_translate::translate_subtitles_with_rate_limit_cancellable(
                vec![translator(url)],
                None,
                source(),
                "it",
                1,
                0,
                None,
                &path,
                |_| {},
                CancellationToken::new(),
            )
            .await
        } else {
            tiered(pool(url), &path, CancellationToken::new(), 1).await
        };
        assert_eq!(result.unwrap()[&1].text, "Ciao");
        server.await.unwrap();
        outputs.push(std::fs::read(&path).unwrap());
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[tokio::test]
async fn persistence_failure_is_returned_instead_of_reporting_translation_success() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing/output.srt");
    let (url, _, release, server) = server().await;
    release.send(()).unwrap();
    let error = tiered(pool(url), &path, CancellationToken::new(), 1)
        .await
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("Failed to save translated subtitles"),
        "{error:#}"
    );
    server.await.unwrap();
    assert!(!path.exists());
}

#[tokio::test]
async fn cancellation_and_abort_stop_inflight_requests_and_prevent_late_writes() {
    for abort in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("output.srt");
        let task_path = path.clone();
        let token = CancellationToken::new();
        let task_token = token.clone();
        let (url, ready, release, server) = server().await;
        let task = tokio::spawn(async move { tiered(pool(url), &task_path, task_token, 1).await });
        ready.await.unwrap();
        if abort {
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        } else {
            token.cancel();
            let error = tokio::time::timeout(Duration::from_secs(2), task)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err();
            assert!(error.to_string().contains("cancelled"));
        }
        release.send(()).unwrap();
        server.await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !path.exists(),
            "A detached worker wrote output after its command ended"
        );
    }
}

#[tokio::test]
async fn worker_panics_are_returned_to_the_caller() {
    let directory = tempfile::tempdir().unwrap();
    let error = srt_translate::translate_subtitles_tiered_cancellable(
        pool("http://127.0.0.1:1/v1".into()),
        source(),
        "it",
        1,
        0,
        None,
        &directory.path().join("output.srt"),
        |_| panic!("callback failure"),
        CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert!(format!("{error:#}").contains("Translation worker failed"));
}

#[tokio::test]
async fn equal_count_with_unrequested_ids_is_not_a_valid_translation() {
    let (url, _, release, server) = server_with_id(99).await;
    release.send(()).unwrap();
    let error = translator(url)
        .translate_batch(&[(1, "Hello".into())], "it", None)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("IDs"));
    server.await.unwrap();
}

#[tokio::test]
async fn repair_failure_does_not_replace_existing_translation_with_original() {
    let mut original = source();
    let subtitle = original.remove(&1).unwrap();
    original.insert(
        u32::MAX,
        Subtitle {
            id: u32::MAX,
            ..subtitle
        },
    );
    let mut translated = original.clone();
    translated.get_mut(&u32::MAX).unwrap().text = "Existing translation".into();
    // Closed local port provides a deterministic transport failure, without a remote API.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    drop(listener);
    let error = srt_translate::repair_translation(
        vec![translator(url)],
        &original,
        &mut translated,
        vec![u32::MAX],
        "it",
        None,
        |_| {},
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("Repair failed"));
    assert_eq!(translated[&u32::MAX].text, "Existing translation");
}
