use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn server(
    response: &'static [u8],
    stall: bool,
) -> (
    String,
    tokio::sync::oneshot::Receiver<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/resource", listener.local_addr().unwrap());
    let (send, ready) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 4096];
        assert!(stream.read(&mut request).await.unwrap() > 0);
        stream.write_all(response).await.unwrap();
        let _ = send.send(());
        if stall {
            std::future::pending::<()>().await;
        }
    });
    (url, ready, task)
}

#[tokio::test]
async fn successful_download_is_atomic_and_cached() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("resource.bin");
    let (url, _, server) = server(
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata",
        false,
    )
    .await;
    let progress = std::sync::Mutex::new(Vec::new());
    download_to(
        &url,
        &path,
        |percent| progress.lock().unwrap().push(percent),
        None,
    )
    .await
    .unwrap();
    server.await.unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"data");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    assert_eq!(progress.into_inner().unwrap(), [0, 100]);
    download_to("invalid://cached", &path, |_| {}, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn http_truncation_and_empty_body_errors_leave_no_partial_file() {
    for response in [
        b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nshort".as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".as_slice(),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("resource.bin");
        let (url, _, server) = server(response, false).await;
        let completed = std::sync::Mutex::new(false);
        assert!(
            download_to(
                &url,
                &path,
                |p| if p == 100 {
                    *completed.lock().unwrap() = true
                },
                None
            )
            .await
            .is_err()
        );
        server.await.unwrap();
        assert!(!completed.into_inner().unwrap());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn cancellation_and_abort_clean_up_while_headers_or_body_are_stalled() {
    for response in [
        b"".as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\npartial".as_slice(),
    ] {
        for abort in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("resource.bin");
            let token = CancellationToken::new();
            let task_token = token.clone();
            let (url, ready, server) = server(response, true).await;
            let task =
                tokio::spawn(
                    async move { download_to(&url, &path, |_| {}, Some(&task_token)).await },
                );
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
            server.abort();
            assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
        }
    }
}

#[tokio::test]
async fn precancelled_and_invalid_cached_resources_do_not_issue_requests() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("resource.bin");
    let token = CancellationToken::new();
    token.cancel();
    assert!(
        download_to("invalid://url", &path, |_| {}, Some(&token))
            .await
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
    std::fs::write(&path, b"").unwrap();
    assert!(
        download_to("invalid://url", &path, |_| {}, None)
            .await
            .unwrap_err()
            .to_string()
            .contains("nonempty")
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"");
}

#[tokio::test]
async fn concurrent_downloads_publish_one_complete_resource() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("resource.bin");
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let mut urls = Vec::new();
    let mut servers = Vec::new();
    for _ in 0..2 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        urls.push(format!(
            "http://{}/resource",
            listener.local_addr().unwrap()
        ));
        let barrier = barrier.clone();
        servers.push(tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            assert!(stream.read(&mut request).await.unwrap() > 0);
            // Neither server responds until both callers have missed the cache.
            barrier.wait().await;
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata")
                .await
                .unwrap();
        }));
    }
    let left = &urls[0];
    let right = &urls[1];
    let (left_result, right_result) = tokio::join!(
        download_to(left, &path, |_| {}, None),
        download_to(right, &path, |_| {}, None),
    );
    left_result.unwrap();
    right_result.unwrap();
    for server in servers {
        server.await.unwrap();
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"data");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}
