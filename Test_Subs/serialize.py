import re
import subprocess
from pathlib import Path


def srt_time_to_seconds(t_str: str) -> float:
    h, m, s_ms = t_str.split(":")
    s, ms = s_ms.split(",")
    return int(h) * 3600 + int(m) * 60 + int(s) + int(ms) / 1000


def seconds_to_srt_time(seconds: float) -> str:
    total_ms = max(0, int(round(seconds * 1000)))
    h = total_ms // 3_600_000
    total_ms %= 3_600_000
    m = total_ms // 60_000
    total_ms %= 60_000
    s = total_ms // 1000
    ms = total_ms % 1000
    return f"{h:02}:{m:02}:{s:02},{ms:03}"


def parse_srt(path: Path):
    text = path.read_text(encoding="utf-8")
    text = text.replace("\r\n", "\n")

    pattern = re.compile(
        r"(\d+)\n"
        r"(\d{2}:\d{2}:\d{2},\d{3})\s*-->\s*(\d{2}:\d{2}:\d{2},\d{3})\n"
        r"(.*?)(?=\n\n\d+\n|\Z)",
        re.DOTALL,
    )

    cues = []
    for match in pattern.finditer(text):
        start = srt_time_to_seconds(match.group(2))
        end = srt_time_to_seconds(match.group(3))
        body = match.group(4).strip("\n")
        cues.append((start, end, body))
    return cues


def write_srt(path: Path, cues):
    lines = []
    for idx, (start, end, body) in enumerate(cues, start=1):
        lines.append(str(idx))
        lines.append(f"{seconds_to_srt_time(start)} --> {seconds_to_srt_time(end)}")
        lines.append(body)
        lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


def split_srt(cues, segment_start: float, segment_end: float):
    out = []
    for cue_start, cue_end, body in cues:
        # Tiene solo la porzione che cade nel segmento, con eventuale clipping ai bordi.
        overlap_start = max(cue_start, segment_start)
        overlap_end = min(cue_end, segment_end)
        if overlap_end <= overlap_start:
            continue

        local_start = overlap_start - segment_start
        local_end = overlap_end - segment_start
        out.append((local_start, local_end, body))
    return out


def split_video_ffmpeg(video_path: Path, out_path: Path, start: float, end: float):
    duration = max(0.0, end - start)
    cmd = [
        "ffmpeg",
        "-y",
        "-ss",
        f"{start:.3f}",
        "-i",
        str(video_path),
        "-t",
        f"{duration:.3f}",
        "-c",
        "copy",
        str(out_path),
    ]
    subprocess.run(cmd, check=True)


def main():
    out_dir = Path("SERIE_TV")
    out_dir.mkdir(exist_ok=True)

    video_file = Path("Detour(1945).mp4")
    srt_files = sorted(Path(".").glob("*.srt"))

    if not video_file.exists():
        raise FileNotFoundError("Video mancante: Detour(1945).mp4")
    if not srt_files:
        raise FileNotFoundError("Nessun file SRT trovato nella directory corrente.")

    # Usa la durata del video per avere 3 parti realmente uguali nel tempo.
    probe_cmd = [
        "ffprobe",
        "-v",
        "error",
        "-show_entries",
        "format=duration",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
        str(video_file),
    ]
    result = subprocess.run(probe_cmd, check=True, capture_output=True, text=True)
    total_duration = float(result.stdout.strip())

    cut_1 = total_duration / 3
    cut_2 = 2 * total_duration / 3
    segments = [
        (0.0, cut_1, "parte1"),
        (cut_1, cut_2, "parte2"),
        (cut_2, total_duration, "parte3"),
    ]

    for seg_start, seg_end, label in segments:
        out_video = out_dir / f"Detour_{label}.mp4"
        split_video_ffmpeg(video_file, out_video, seg_start, seg_end)

        for srt_path in srt_files:
            cues = parse_srt(srt_path)
            split_cues = split_srt(cues, seg_start, seg_end)
            out_srt = out_dir / f"{srt_path.stem}_{label}.srt"
            write_srt(out_srt, split_cues)

    print("Completato: create 3 parti video e SRT in SERIE_TV/")


if __name__ == "__main__":
    main()