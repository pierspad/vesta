# srt-extract

Library for extracting, summarizing, and formatting subtitle data parsed by `srt-parser`.

It powers the corresponding headless CLI and supports structured JSON and human-readable output.

```bash
cargo test -p srt-extract
```

License: GPL-3.0.

`embedded::list_embedded_subtitles` probes media-container tracks;
`embedded::extract_embedded_subtitle` saves a selected text track as SRT through
FFmpeg. Bitmap subtitles need OCR. Probe/extraction have finite timeouts and
failed extraction preserves an existing output. FFmpeg/ffprobe are required only
for embedded tracks; ordinary parsed-SRT conversion stays in memory.
