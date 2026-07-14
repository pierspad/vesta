# Vesta

**subs2srs, but actually fast.**

Vesta is a desktop app for turning video files into translated subtitles and Anki decks. 
If you've used subs2srs, the workflow will feel immediately familiar; same core idea, rebuilt from scratch to be faster and less painful to use.

Built with Rust (Tauri) + Svelte.

---

## What it does

Load a video. Get subtitles. Translate them. Export an Anki deck with video clips, audio snippets, and screenshot cards all synced to the exact lines of dialogue. The whole pipeline that used to take an hour now takes a few minutes.

![Benchmark comparison: Vesta vs subs2srs](benchmark_comparison.png)

Benchmarks were run on an Intel i5-1135G7 laptop CPU (4 cores, 8 threads). Even on that modest CPU, Vesta is consistently much faster than subs2srs for flashcard generation because it is written in Rust and parallelizes the expensive work across available cores: subtitle parsing and matching, media extraction orchestration, TSV/APKG generation, and file output.

On the benchmark set, Vesta completes the same flashcard-generation workflow in roughly **2.3-2.6× less time** than subs2srs. On CPUs with more cores and higher sustained performance, the gap should become even clearer because Vesta has more parallel work available than the classic subs2srs pipeline.

---

## Core Feature

**Flashcards** — generates Anki decks from your subtitles. 
You can also export it directly in .apkg format to import in Anki.
Each card can also include:
- an audio snippet
- a snapshot of the sentence
- a video clip of the sentence

## More Features

**Translation**: If you have the original subtitle file and you cannot really find the subtitle in your language, you can translate it using an LLM.
Either connect to an existing API or run your own instance locally.

**Sync**: If your srt file is not in sync with the audio, you can sync it using an interactive wizard.
You can either use the automatic sync that will try to put anchors using Whisper, or you can put the anchors manually.
The ideal workflow is to use Whisper to find the rough timestamps and then manually adjust them.
The anchors put by the user have an higher priority in fidelity than the anchors put by Whisper.

**Revision**: A built-in SRT editor for when you want to clean things up by hand.

**Transcription**: If you lack also the original srt file you can use Vesta to generate SRT subtitles straight from the audio using Whisper locally. 
It is strictly recommended to use this feature only if you really don't have the subtitle file, since the quality of the generated srt is not always perfect as a human vetted one.

---

## Pipeline

You don't have to start from scratch. Jump in at whatever step makes sense:

```
Video → [Transcribe] → [Sync] → [Translate] → [Flashcards]
```

Already have an SRT? Skip straight to Sync or Flashcards.

---

## Modular & headless use

Vesta is a workspace of decoupled crates: every feature is a GUI-agnostic
library (`lib/`) with a matching command-line front-end (`cli/`), and the
desktop app is just a thin adapter on top. If you have a better idea for a
subs2srs successor but don't want to rewrite the machinery, take the module
you need — as a standalone binary or as a Rust dependency — and build on it:

```bash
cargo build --release -p srt-flashcards-cli   # subtitles + video → Anki deck (TSV/APKG)
cargo build --release -p srt-transcribe-cli    # media → SRT (whisper.cpp or cloud)
cargo build --release -p srt-autosync-cli      # auto re-sync an SRT via Whisper anchors
cargo build --release -p srt-translate-cli     # LLM subtitle translation
cargo build --release -p srt-extract-cli       # SRT data extraction

target/release/srt-flashcards generate \
  --target movie-en.srt --native movie-it.srt --video movie.mp4 --output out --format apkg
```

Each module has its own guide — what it does, how to build just its binary,
and how to embed it in your own Rust project:

- [Modules overview](modules/README.md) — the map of crates and the design contract
- [srt-parser](modules/srt-parser.md) — SRT parsing & writing
- [srt-extract](modules/srt-extract.md) — subtitle data extraction (JSON, stats…)
- [srt-translate](modules/srt-translate.md) — LLM translation with multi-tier failover
- [srt-sync](modules/srt-sync.md) — anchor-based re-timing engine
- [srt-autosync](modules/srt-autosync.md) — automatic alignment via Whisper anchors
- [whisper-common](modules/whisper-common.md) — transcription pipeline (media → SRT)
- [srt-flashcards](modules/srt-flashcards.md) — subs2srs-style Anki deck generation
- [srt-refine](modules/srt-refine.md) — LLM enrichment of existing decks

See also [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) for the layer rules that
keep these crates extractable.

## Benchmarks (reproducible)

The chart above can be regenerated — and Vesta variants compared — with the
numbered scripts in [`benchmarking_against_subs2srs/`](../benchmarking_against_subs2srs/):

```bash
./benchmarking_against_subs2srs/1_compile_subs2srs.sh   # headless subs2srs harness (its real code, no GUI)
./benchmarking_against_subs2srs/2_compile_vesta.sh      # the Vesta flashcard CLI (release)
./benchmarking_against_subs2srs/3_run_benchmarks.sh     # time both on the test media
./benchmarking_against_subs2srs/4_generate_report.sh    # chart + summary
```

Vesta runs on `cores-1` workers; subs2srs runs exactly as written
(single-threaded). Same inputs, same outputs, pure execution-time comparison —
see [`benchmarking_against_subs2srs/README.md`](../benchmarking_against_subs2srs/README.md).

---

## Test media

Development was done using the public domain film **Detour (1945)** — good length, clear dialogue, freely available.

→ [Download Detour (1945) HD on archive.org](https://archive.org/details/detour1945HD)

---

## Series naming convention

When processing multiple episodes, name your files so Vesta can automatically detect season and episode numbers:

```
name_[season<N>]_[ep]<N>.ext
```
or the simpler:
```
<name>_S<N>E<N>.ext
```

**Examples:**
```
12_angry_men_[season01]_[ep]01.mp4
breaking_bad_s01e05.mp4
```

The bracketed format exists specifically for titles that start with numbers (like "12 Angry Men"), so Vesta doesn't confuse the title with episode metadata. Exported decks will come out as `<DeckName>_<Episode>.apkg`, one per episode.
