//! Bounded file responses for the desktop stream protocol, independent of bootstrap.
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
use tauri::http::{Request, Response, StatusCode};

const INITIAL_LIMIT: u64 = 2 * 1024 * 1024;
const RANGE_LIMIT: u64 = 4 * 1024 * 1024;

pub fn response<B>(request: &Request<B>) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let encoded = uri
        .strip_prefix("stream://localhost/")
        .or_else(|| uri.strip_prefix("stream://localhost"))
        .unwrap_or("");
    let path = urlencoding::decode(encoded).unwrap_or_else(|_| encoded.into());
    let range = request
        .headers()
        .get("range")
        .and_then(|header| header.to_str().ok());
    match serve(Path::new(path.as_ref()), range) {
        Ok(response) => response,
        Err(error) => Response::builder()
            .status(if error.kind() == std::io::ErrorKind::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            })
            .header("Content-Type", "text/plain")
            .body(format!("Unable to read media: {error}").into_bytes())
            .expect("Static media error headers are valid"),
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
        "opus" => "audio/ogg",
        "wav" | "wave" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
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

    fn request(path: &Path, range: Option<&str>) -> Request<()> {
        let mut request = Request::builder().uri(format!(
            "stream://localhost/{}",
            urlencoding::encode(path.to_str().unwrap())
        ));
        if let Some(range) = range {
            request = request.header("Range", range);
        }
        request.body(()).unwrap()
    }

    #[test]
    fn encoded_paths_ranges_and_empty_files_have_consistent_headers_and_body() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("日本語 clip.MP3");
        std::fs::write(&path, b"0123456789").unwrap();
        let full = response(&request(&path, None));
        assert_eq!(full.status(), StatusCode::OK);
        assert_eq!(full.headers()["Content-Type"], "audio/mpeg");
        assert_eq!(full.body(), b"0123456789");
        for (range, body, header) in [
            ("bytes=2-4", b"234".as_slice(), "bytes 2-4/10"),
            ("bytes=-3", b"789".as_slice(), "bytes 7-9/10"),
        ] {
            let partial = response(&request(&path, Some(range)));
            assert_eq!(partial.status(), StatusCode::PARTIAL_CONTENT);
            assert_eq!(partial.headers()["Content-Range"], header);
            assert_eq!(partial.body(), body);
        }
        let invalid = response(&request(&path, Some("bytes=10-")));
        assert_eq!(invalid.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(invalid.headers()["Content-Range"], "bytes */10");
        std::fs::write(&path, []).unwrap();
        let empty = response(&request(&path, None));
        assert_eq!(empty.status(), StatusCode::OK);
        assert_eq!(empty.headers()["Content-Length"], "0");
        assert_eq!(
            response(&request(&path, Some("bytes=0-"))).status(),
            StatusCode::RANGE_NOT_SATISFIABLE
        );
        assert_eq!(
            response(&request(&directory.path().join("missing"), None)).status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn large_media_responses_remain_bounded_and_report_actual_range() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("film.mp4");
        let file = File::create(&path).unwrap();
        file.set_len(RANGE_LIMIT + 1024).unwrap();
        for (range, expected_size) in [(None, INITIAL_LIMIT), (Some("bytes=0-"), RANGE_LIMIT)] {
            let response = response(&request(&path, range));
            assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
            assert_eq!(response.body().len() as u64, expected_size);
            assert_eq!(
                response.headers()["Content-Range"],
                format!("bytes 0-{}/{}", expected_size - 1, RANGE_LIMIT + 1024)
            );
        }
    }
}
