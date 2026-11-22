//! Bounded file responses for the desktop stream protocol and local media server.
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use axum::{
    Router,
    extract::{Query, Request as AxumRequest, State as AxumState},
    http::{HeaderValue, Method, StatusCode as AxumStatusCode, header},
    response::IntoResponse,
    routing::get,
};
use subtle::ConstantTimeEq;
use tauri::http::{Request, Response, StatusCode};
use tower::ServiceExt;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::ServeFile,
};

const INITIAL_LIMIT: u64 = 2 * 1024 * 1024;
const RANGE_LIMIT: u64 = 4 * 1024 * 1024;

#[derive(Clone, Default)]
pub struct MediaAllowlist {
    allowed_files: Arc<RwLock<HashSet<PathBuf>>>,
}

impl MediaAllowlist {
    pub fn allow_file(&self, path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
        let requested = path.as_ref();
        if requested.as_os_str().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Empty media path",
            ));
        }
        let link_meta = std::fs::symlink_metadata(requested)?;
        if link_meta.file_type().is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Symlink media paths are not allowed",
            ));
        }
        let canonical = std::fs::canonicalize(requested)?;
        let meta = std::fs::metadata(&canonical)?;
        if !meta.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Media path is not a regular file",
            ));
        }
        let mut guard = self
            .allowed_files
            .write()
            .unwrap_or_else(|err| err.into_inner());
        guard.insert(canonical.clone());
        Ok(canonical)
    }

    pub fn resolve_allowed(&self, requested: &Path) -> Option<PathBuf> {
        if requested.as_os_str().is_empty() || !requested.is_absolute() {
            return None;
        }
        if std::fs::symlink_metadata(requested)
            .ok()?
            .file_type()
            .is_symlink()
        {
            return None;
        }
        let canonical = std::fs::canonicalize(requested).ok()?;
        if !std::fs::metadata(&canonical).ok()?.is_file() {
            return None;
        }
        let guard = self
            .allowed_files
            .read()
            .unwrap_or_else(|err| err.into_inner());
        guard.contains(&canonical).then_some(canonical)
    }

    #[cfg(test)]
    pub fn is_allowed(&self, requested: &Path) -> bool {
        self.resolve_allowed(requested).is_some()
    }
}

#[derive(Clone)]
pub struct MediaAuthState {
    pub token: String,
    pub allowlist: MediaAllowlist,
}

pub fn generate_media_token() -> Result<String, getrandom::Error> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    let mut token = String::with_capacity(64);
    for byte in bytes {
        token.push(HEX[(byte >> 4) as usize] as char);
        token.push(HEX[(byte & 0x0f) as usize] as char);
    }
    Ok(token)
}

pub fn verify_token(provided: &str, expected: &str) -> bool {
    if expected.is_empty() {
        return false;
    }
    bool::from(provided.as_bytes().ct_eq(expected.as_bytes()))
}

pub fn bind_media_listener(addr: &str) -> std::io::Result<(std::net::TcpListener, u16)> {
    let listener = std::net::TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    Ok((listener, port))
}

#[derive(serde::Deserialize)]
pub struct MediaParams {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub token: String,
}

pub fn build_media_cors() -> CorsLayer {
    let origins = [
        HeaderValue::from_static("tauri://localhost"),
        HeaderValue::from_static("http://tauri.localhost"),
        HeaderValue::from_static("https://tauri.localhost"),
        HeaderValue::from_static("http://localhost:5175"),
        HeaderValue::from_static("http://127.0.0.1:5175"),
    ];
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::HEAD])
        .allow_headers([header::RANGE, header::CONTENT_TYPE])
}

pub async fn media_handler(
    AxumState(auth): AxumState<MediaAuthState>,
    Query(params): Query<MediaParams>,
    req: AxumRequest,
) -> Result<impl IntoResponse, AxumStatusCode> {
    if !verify_token(&params.token, &auth.token) {
        return Err(AxumStatusCode::FORBIDDEN);
    }
    if params.path.is_empty() {
        return Err(AxumStatusCode::BAD_REQUEST);
    }
    let Some(canonical) = auth.allowlist.resolve_allowed(Path::new(&params.path)) else {
        return Err(AxumStatusCode::FORBIDDEN);
    };
    ServeFile::new(canonical)
        .oneshot(req)
        .await
        .map_err(|_| AxumStatusCode::INTERNAL_SERVER_ERROR)
}

pub fn build_media_router(auth: MediaAuthState) -> Router {
    Router::new()
        .route("/media", get(media_handler))
        .with_state(auth)
        .layer(build_media_cors())
}

fn status_text_response(status: StatusCode, body: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(body.as_bytes().to_vec())
        .expect("Static media error headers are valid")
}

fn strip_stream_origin(uri: &str) -> Option<&str> {
    for prefix in [
        "stream://localhost",
        "http://stream.localhost",
        "https://stream.localhost",
    ] {
        if let Some(after_host) = uri.strip_prefix(prefix) {
            if let Some(rest) = after_host.strip_prefix('/') {
                return Some(rest);
            }
            if after_host.is_empty() || after_host.starts_with('?') || after_host.starts_with('#') {
                return Some(after_host);
            }
        }
    }
    None
}

fn extract_stream_token(raw_query: &str) -> Option<String> {
    let mut found: Option<String> = None;
    for pair in raw_query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_val) = pair.split_once('=').unwrap_or((pair, ""));
        let key = urlencoding::decode(raw_key).ok()?;
        if key == "token" {
            if found.is_some() {
                return None;
            }
            let val = urlencoding::decode(raw_val).ok()?;
            found = Some(val.into_owned());
        }
    }
    found
}

pub fn response<B>(request: &Request<B>, auth: &MediaAuthState) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let Some(rest) = strip_stream_origin(&uri) else {
        return status_text_response(StatusCode::FORBIDDEN, "Forbidden");
    };
    let without_fragment = rest.split('#').next().unwrap_or("");
    let (raw_path, raw_query) = without_fragment
        .split_once('?')
        .unwrap_or((without_fragment, ""));
    let Some(provided_token) = extract_stream_token(raw_query) else {
        return status_text_response(StatusCode::FORBIDDEN, "Forbidden");
    };
    if !verify_token(&provided_token, &auth.token) {
        return status_text_response(StatusCode::FORBIDDEN, "Forbidden");
    }
    let Ok(decoded_path) = urlencoding::decode(raw_path) else {
        return status_text_response(StatusCode::BAD_REQUEST, "Invalid media path encoding");
    };
    #[cfg(windows)]
    let normalized_path = {
        let s = decoded_path.as_ref();
        let bytes = s.as_bytes();
        if bytes.len() >= 3
            && bytes[0] == b'/'
            && bytes[1].is_ascii_alphabetic()
            && bytes[2] == b':'
        {
            &s[1..]
        } else {
            s
        }
    };
    #[cfg(not(windows))]
    let normalized_path = decoded_path.as_ref();

    let Some(canonical_path) = auth.allowlist.resolve_allowed(Path::new(normalized_path)) else {
        return status_text_response(StatusCode::FORBIDDEN, "Forbidden");
    };

    let range = request
        .headers()
        .get("range")
        .and_then(|header| header.to_str().ok());
    match serve(&canonical_path, range) {
        Ok(response) => response,
        Err(error) => status_text_response(
            if error.kind() == std::io::ErrorKind::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            },
            &format!("Unable to read media: {error}"),
        ),
    }
}

fn serve(path: &Path, range: Option<&str>) -> std::io::Result<Response<Vec<u8>>> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Not a media file",
        ));
    }
    let size = metadata.len();
    let (start, read_size) = if let Some(range) = range {
        let Some((start, end)) = crate::media_range::parse_byte_range(range, size) else {
            return Ok(Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header("Content-Range", format!("bytes */{size}"))
                .body(Vec::new())
                .expect("Numeric range header is valid"));
        };
        (start, (end - start + 1).min(RANGE_LIMIT))
    } else {
        (0, size.min(INITIAL_LIMIT))
    };
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::with_capacity(read_size as usize);
    file.take(read_size).read_to_end(&mut bytes)?;
    if bytes.is_empty() && size > 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Media changed during read",
        ));
    }
    let partial = range.is_some() || (bytes.len() as u64) < size;
    let mut builder = Response::builder()
        .status(if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header("Content-Type", mime_from_ext(path))
        .header("Accept-Ranges", "bytes")
        .header("Content-Length", bytes.len());
    if partial {
        let end = start + bytes.len() as u64 - 1;
        builder = builder.header("Content-Range", format!("bytes {start}-{end}/{size}"));
    }
    Ok(builder
        .body(bytes)
        .expect("Static MIME and numeric media headers are valid"))
}

fn mime_from_ext(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "ogv" => "video/ogg",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "mp3" => "audio/mpeg",
        "opus" | "ogg" | "oga" => "audio/ogg",
        "wav" | "wave" => "audio/wav",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "wma" => "audio/x-ms-wma",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_auth(allowlist: MediaAllowlist) -> MediaAuthState {
        MediaAuthState {
            token: generate_media_token().unwrap(),
            allowlist,
        }
    }

    fn request_uri(uri: &str, range: Option<&str>) -> Request<()> {
        let mut request = Request::builder().uri(uri);
        if let Some(range) = range {
            request = request.header("Range", range);
        }
        request.body(()).unwrap()
    }

    fn request(path: &Path, token: &str, range: Option<&str>) -> Request<()> {
        let uri = format!(
            "stream://localhost/{}?token={}",
            urlencoding::encode(path.to_str().unwrap()),
            urlencoding::encode(token)
        );
        request_uri(&uri, range)
    }

    #[test]
    fn generate_media_token_has_csprng_length_hex_and_uniqueness() {
        let mut tokens = HashSet::new();
        for _ in 0..16 {
            let token = generate_media_token().unwrap();
            assert_eq!(token.len(), 64);
            assert!(
                token
                    .bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            );
            assert!(tokens.insert(token));
        }
        assert_eq!(tokens.len(), 16);
    }

    #[test]
    fn constant_time_token_eq_handles_match_mismatch_and_length_differences() {
        assert!(verify_token("abc123", "abc123"));
        assert!(!verify_token("abc123", "abc124"));
        assert!(!verify_token("abc123", "bbc123"));
        assert!(!verify_token("abc", "abcd"));
        assert!(!verify_token("abcd", "abc"));
        assert!(!verify_token("", "abc"));
        assert!(!verify_token("", ""));
    }

    #[test]
    fn media_allowlist_canonicalizes_files_and_rejects_symlinks_and_missing_paths() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let media_file = dir.path().join("media.mp4");
        let other_file = dir.path().join("other.mp4");
        let subdir = dir.path().join("subdir");
        let missing = dir.path().join("missing.mp4");
        let outside_secret = outside.path().join("secret.txt");

        std::fs::write(&media_file, b"video").unwrap();
        std::fs::write(&other_file, b"other").unwrap();
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(&outside_secret, b"secret").unwrap();

        let allowlist = MediaAllowlist::default();
        assert!(allowlist.allow_file(&missing).is_err());
        assert!(!allowlist.is_allowed(&missing));
        assert!(allowlist.allow_file(&subdir).is_err());
        assert!(!allowlist.is_allowed(&subdir));
        assert!(allowlist.allow_file("").is_err());

        let canonical = allowlist.allow_file(&media_file).unwrap();
        assert!(allowlist.is_allowed(&media_file));
        assert!(allowlist.is_allowed(&canonical));
        assert!(allowlist.is_allowed(&subdir.join("..").join("media.mp4")));
        assert!(!allowlist.is_allowed(&other_file));
        assert!(!allowlist.is_allowed(Path::new("media.mp4")));

        #[cfg(unix)]
        {
            let symlink_outside = dir.path().join("symlink_outside.mp4");
            let symlink_inside = dir.path().join("symlink_inside.mp4");
            std::os::unix::fs::symlink(&outside_secret, &symlink_outside).unwrap();
            std::os::unix::fs::symlink(&media_file, &symlink_inside).unwrap();

            assert!(allowlist.allow_file(&symlink_outside).is_err());
            assert!(!allowlist.is_allowed(&symlink_outside));
            assert!(!allowlist.is_allowed(&outside_secret));
            assert!(allowlist.allow_file(&symlink_inside).is_err());
            assert!(!allowlist.is_allowed(&symlink_inside));
        }
    }

    #[tokio::test]
    async fn axum_media_router_enforces_token_allowlist_and_cors() {
        let dir = tempfile::tempdir().unwrap();
        let allowed = dir.path().join("allowed.mp4");
        let secret = dir.path().join("secret.txt");
        std::fs::write(&allowed, b"hello-media").unwrap();
        std::fs::write(&secret, b"top-secret").unwrap();

        let allowlist = MediaAllowlist::default();
        allowlist.allow_file(&allowed).unwrap();
        let token = generate_media_token().unwrap();
        let router = build_media_router(MediaAuthState {
            token: token.clone(),
            allowlist,
        });

        let allowed_enc = urlencoding::encode(allowed.to_str().unwrap());
        let secret_enc = urlencoding::encode(secret.to_str().unwrap());

        // Valid token + allowlisted file -> 200 OK
        let resp = router
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/media?path={allowed_enc}&token={token}"))
                    .header("Origin", "http://tauri.localhost")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxumStatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|v| v.to_str().ok()),
            Some("http://tauri.localhost")
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"hello-media");

        // Missing or empty or wrong token -> 403 Forbidden
        for uri in [
            format!("/media?path={allowed_enc}"),
            format!("/media?path={allowed_enc}&token="),
            format!("/media?path={allowed_enc}&token=wrong"),
            "/media".to_string(),
        ] {
            let resp = router
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .uri(uri)
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), AxumStatusCode::FORBIDDEN);
        }

        // Valid token + empty path -> 400 Bad Request
        let resp = router
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/media?path=&token={token}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxumStatusCode::BAD_REQUEST);

        // Valid token + unallowlisted / non-existent / traversal path -> 403 Forbidden
        let traversal_raw = format!("{}/../../etc/passwd", dir.path().display());
        let traversal = urlencoding::encode(&traversal_raw);
        let etc_passwd = urlencoding::encode("/etc/passwd");
        for bad_path in [
            secret_enc.as_ref(),
            etc_passwd.as_ref(),
            traversal.as_ref(),
            "relative.mp4",
        ] {
            let resp = router
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .uri(format!("/media?path={bad_path}&token={token}"))
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), AxumStatusCode::FORBIDDEN);
        }

        // Disallowed CORS origin gets no Access-Control-Allow-Origin
        let resp = router
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/media?path={allowed_enc}&token={token}"))
                    .header("Origin", "https://evil.example")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_none()
        );

        // Preflight method/header restriction
        let preflight = router
            .oneshot(
                axum::http::Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/media")
                    .header("Origin", "tauri://localhost")
                    .header("Access-Control-Request-Method", "DELETE")
                    .header("Access-Control-Request-Headers", "X-Evil")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            preflight
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|v| v.to_str().ok()),
            Some("tauri://localhost")
        );
        let allow_methods = preflight
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_METHODS)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(!allow_methods.contains("DELETE"));
        let allow_headers = preflight
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(!allow_headers.to_ascii_lowercase().contains("x-evil"));
    }

    #[test]
    fn bind_media_listener_handles_bind_errors_without_panicking() {
        let (listener, port) = bind_media_listener("127.0.0.1:0").unwrap();
        assert!(port > 0);
        drop(listener);
        assert!(bind_media_listener("256.256.256.256:0").is_err());
    }

    #[test]
    fn stream_rejects_missing_and_wrong_token() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clip.mp3");
        std::fs::write(&path, b"abc").unwrap();
        let allowlist = MediaAllowlist::default();
        allowlist.allow_file(&path).unwrap();
        let auth = test_auth(allowlist);
        let encoded = urlencoding::encode(path.to_str().unwrap());

        for uri in [
            format!("stream://localhost/{encoded}"),
            format!("stream://localhost/{encoded}?token="),
            format!("stream://localhost/{encoded}?token=wrong"),
            format!(
                "stream://localhost/{encoded}?token={}&token={}",
                auth.token, auth.token
            ),
            format!("stream://localhost.evil.com/{encoded}?token={}", auth.token),
        ] {
            let resp = response(&request_uri(&uri, None), &auth);
            assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        }
    }

    #[test]
    fn stream_rejects_unallowlisted_path_even_with_valid_token() {
        let dir = tempfile::tempdir().unwrap();
        let secret = dir.path().join("secret.txt");
        std::fs::write(&secret, b"secret").unwrap();
        let auth = test_auth(MediaAllowlist::default());

        assert_eq!(
            response(&request(&secret, &auth.token, None), &auth).status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            response(&request(Path::new("/etc/passwd"), &auth.token, None), &auth).status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn stream_rejects_path_traversal_and_query_injection() {
        let dir = tempfile::tempdir().unwrap();
        let allowed = dir.path().join("clip.mp3");
        std::fs::write(&allowed, b"audio-bytes").unwrap();
        let allowlist = MediaAllowlist::default();
        allowlist.allow_file(&allowed).unwrap();
        let auth = test_auth(allowlist);

        let traversal = format!("{}/../../etc/passwd", dir.path().display());
        assert_eq!(
            response(
                &request_uri(
                    &format!(
                        "stream://localhost/{}?token={}",
                        urlencoding::encode(&traversal),
                        auth.token
                    ),
                    None
                ),
                &auth
            )
            .status(),
            StatusCode::FORBIDDEN
        );

        // Encoded `?token=` inside the path segment must not satisfy query auth
        let smuggled_raw = format!("{}?token={}", allowed.display(), auth.token);
        let smuggled = urlencoding::encode(&smuggled_raw);
        assert_eq!(
            response(
                &request_uri(&format!("stream://localhost/{smuggled}"), None),
                &auth
            )
            .status(),
            StatusCode::FORBIDDEN
        );

        // Extra query param `path=` must not override the request path before `?`
        let allowed_enc = urlencoding::encode(allowed.to_str().unwrap());
        let ok_resp = response(
            &request_uri(
                &format!(
                    "stream://localhost/{allowed_enc}?foo=1&token={}&path=/etc/passwd#t=0,1",
                    auth.token
                ),
                None,
            ),
            &auth,
        );
        assert_eq!(ok_resp.status(), StatusCode::OK);
        assert_eq!(ok_resp.body(), b"audio-bytes");
    }

    #[test]
    fn stream_supports_http_stream_localhost_and_rejects_malformed_encoding() {
        let dir = tempfile::tempdir().unwrap();
        let allowed = dir.path().join("clip.mp3");
        std::fs::write(&allowed, b"0123456789").unwrap();
        let allowlist = MediaAllowlist::default();
        allowlist.allow_file(&allowed).unwrap();
        let auth = test_auth(allowlist);
        let allowed_enc = urlencoding::encode(allowed.to_str().unwrap());

        let http_resp = response(
            &request_uri(
                &format!("http://stream.localhost/{allowed_enc}?token={}", auth.token),
                Some("bytes=1-3"),
            ),
            &auth,
        );
        assert_eq!(http_resp.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(http_resp.body(), b"123");

        let https_resp = response(
            &request_uri(
                &format!(
                    "https://stream.localhost/{allowed_enc}?token={}",
                    auth.token
                ),
                None,
            ),
            &auth,
        );
        assert_eq!(https_resp.status(), StatusCode::OK);
        assert_eq!(https_resp.body(), b"0123456789");

        // Malformed UTF-8 percent encoding -> 400 Bad Request
        let bad_enc = response(
            &request_uri(
                &format!("stream://localhost/%FF%FE?token={}", auth.token),
                None,
            ),
            &auth,
        );
        assert_eq!(bad_enc.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn encoded_paths_ranges_and_empty_files_have_consistent_headers_and_body() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("日本語 clip.MP3");
        std::fs::write(&path, b"0123456789").unwrap();
        let allowlist = MediaAllowlist::default();
        allowlist.allow_file(&path).unwrap();
        let auth = test_auth(allowlist);

        let full = response(&request(&path, &auth.token, None), &auth);
        assert_eq!(full.status(), StatusCode::OK);
        assert_eq!(full.headers()["Content-Type"], "audio/mpeg");
        assert_eq!(full.body(), b"0123456789");
        for (range, body, header) in [
            ("bytes=2-4", b"234".as_slice(), "bytes 2-4/10"),
            ("bytes=-3", b"789".as_slice(), "bytes 7-9/10"),
        ] {
            let partial = response(&request(&path, &auth.token, Some(range)), &auth);
            assert_eq!(partial.status(), StatusCode::PARTIAL_CONTENT);
            assert_eq!(partial.headers()["Content-Range"], header);
            assert_eq!(partial.body(), body);
        }
        let invalid = response(&request(&path, &auth.token, Some("bytes=10-")), &auth);
        assert_eq!(invalid.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(invalid.headers()["Content-Range"], "bytes */10");
        std::fs::write(&path, []).unwrap();
        let empty = response(&request(&path, &auth.token, None), &auth);
        assert_eq!(empty.status(), StatusCode::OK);
        assert_eq!(empty.headers()["Content-Length"], "0");
        assert_eq!(
            response(&request(&path, &auth.token, Some("bytes=0-")), &auth).status(),
            StatusCode::RANGE_NOT_SATISFIABLE
        );
        assert_eq!(
            response(
                &request(&directory.path().join("missing"), &auth.token, None),
                &auth
            )
            .status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn large_media_responses_remain_bounded_and_report_actual_range() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("film.mp4");
        let file = File::create(&path).unwrap();
        file.set_len(RANGE_LIMIT + 1024).unwrap();
        let allowlist = MediaAllowlist::default();
        allowlist.allow_file(&path).unwrap();
        let auth = test_auth(allowlist);

        for (range, expected_size) in [(None, INITIAL_LIMIT), (Some("bytes=0-"), RANGE_LIMIT)] {
            let response = response(&request(&path, &auth.token, range), &auth);
            assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
            assert_eq!(response.body().len() as u64, expected_size);
            assert_eq!(
                response.headers()["Content-Range"],
                format!("bytes 0-{}/{}", expected_size - 1, RANGE_LIMIT + 1024)
            );
        }
    }
}
