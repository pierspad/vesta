use srt_translate::{ApiType, Translator, TranslatorConfig};
use srt_translate::{ProviderError, ProviderFailureKind, TranslationDiagnostic};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn server(responses: Vec<(u16, String)>) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        for (status, body) in responses {
            let (mut stream, _) =
                tokio::time::timeout(std::time::Duration::from_secs(10), listener.accept())
                    .await
                    .unwrap()
                    .unwrap();
            let mut bytes = [0; 8192];
            assert!(stream.read(&mut bytes).await.unwrap() > 0);
            stream.write_all(format!("HTTP/1.1 {status} Response\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    (url, task)
}

fn translator(url: String, api_type: ApiType) -> Translator {
    Translator::new(TranslatorConfig {
        base_url: url,
        model: "test".into(),
        api_key: Some("private-key".into()),
        api_type,
    })
}

#[tokio::test]
async fn http_failure_never_becomes_success_even_with_a_success_shaped_body() {
    for (kind, body) in [
        (
            ApiType::Local,
            r#"{"choices":[{"message":{"role":"assistant","content":"not a success"}}]}"#,
        ),
        (
            ApiType::Google,
            r#"{"candidates":[{"content":{"parts":[{"text":"not a success"}]}}]}"#,
        ),
    ] {
        let (url, task) = server(vec![(401, body.into())]).await;
        let result = translator(url, kind).generate_response("prompt").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("401"));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn malformed_provider_responses_do_not_echo_credentials_or_private_text() {
    for kind in [ApiType::Local, ApiType::Google] {
        let (url, task) = server(vec![(
            200,
            "private-key PRIVATE-SUBTITLE <html>unexpected proxy</html>".into(),
        )])
        .await;
        let message = translator(url, kind)
            .generate_response("prompt")
            .await
            .unwrap_err()
            .to_string();
        assert!(message.contains("200"));
        assert!(!message.contains("private-key"));
        assert!(!message.contains("PRIVATE-SUBTITLE"));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn empty_success_is_an_error_instead_of_a_valid_annotation() {
    let (url, task) = server(vec![(200, r#"{"choices":[]}"#.into())]).await;
    assert!(
        translator(url, ApiType::Local)
            .generate_response("prompt")
            .await
            .is_err()
    );
    task.await.unwrap();
}

#[tokio::test]
async fn retry_events_are_optional_safe_and_retained_by_clones() {
    let (url, task) = server(vec![
        (429, "private-key quota exceeded; retry in 0s".into()),
        (
            200,
            r#"{"choices":[{"message":{"role":"assistant","content":"Recovered"}}]}"#.into(),
        ),
    ])
    .await;
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let received = events.clone();
    let original = translator(url, ApiType::Local)
        .with_diagnostics(move |event| received.lock().unwrap().push(event));
    assert_eq!(
        original
            .clone()
            .generate_response("private prompt")
            .await
            .unwrap(),
        "Recovered"
    );
    task.await.unwrap();
    assert_eq!(
        *events.lock().unwrap(),
        vec![TranslationDiagnostic {
            status: 429,
            attempt: 1,
            max_retries: 3,
            retry_in_ms: 3000
        }]
    );
}

#[tokio::test]
async fn provider_errors_have_typed_status_and_safe_categories() {
    for (status, body, kind) in [
        (
            401,
            "private-key unauthorized",
            ProviderFailureKind::Authentication,
        ),
        (
            403,
            "private-key RESOURCE_EXHAUSTED quota",
            ProviderFailureKind::RateLimit,
        ),
        (400, "PRIVATE-SUBTITLE", ProviderFailureKind::InvalidRequest),
    ] {
        let (url, task) = server(vec![(status, body.into())]).await;
        let error = translator(url, ApiType::Local)
            .generate_response("prompt")
            .await
            .unwrap_err();
        let detail = error.downcast_ref::<ProviderError>().unwrap();
        assert_eq!(detail.status, status);
        assert_eq!(detail.kind, kind);
        assert!(!error.to_string().contains("private-key"));
        assert!(!error.to_string().contains("PRIVATE-SUBTITLE"));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn transport_errors_do_not_include_credential_bearing_urls() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/private-key?token=private-token",
        listener.local_addr().unwrap()
    );
    drop(listener);
    let error = translator(url, ApiType::Local)
        .generate_response("prompt")
        .await
        .unwrap_err();
    assert!(!format!("{error:#}").contains("private-key"));
    assert!(!format!("{error:#}").contains("private-token"));
}

#[tokio::test]
async fn normalized_empty_translations_are_rejected_in_both_entry_points() {
    for content in ["\"\"", "\"   \""] {
        for contextual in [false, true] {
            let body =
                serde_json::json!({"choices":[{"message":{"role":"assistant","content":content}}]})
                    .to_string();
            let (url, task) = server(vec![(200, body)]).await;
            let translator = translator(url, ApiType::Local);
            let result = if contextual {
                translator
                    .translate_with_context("Hello", "it", None, None)
                    .await
            } else {
                translator.translate("Hello", "it", None).await
            };
            assert!(
                result.is_err(),
                "accepted normalized empty response: {content}"
            );
            task.await.unwrap();
        }
    }
}

#[tokio::test]
async fn batch_rejects_empty_text_and_duplicate_ids_without_echoing_text() {
    for content in [
        r#"[{"id":1,"text":""}]"#,
        r#"[{"id":1,"text":"   "}]"#,
        r#"[{"id":1,"text":"PRIVATE-first"},{"id":1,"text":"PRIVATE-second"}]"#,
        "ID: 1 | Translation:",
    ] {
        let body =
            serde_json::json!({"choices":[{"message":{"role":"assistant","content":content}}]})
                .to_string();
        let (url, task) = server(vec![(200, body)]).await;
        let error = translator(url, ApiType::Local)
            .translate_batch(&[(1, "Hello".into())], "it", None)
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("PRIVATE"));
        task.await.unwrap();
    }
}
