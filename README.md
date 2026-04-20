# Vesta

**subs2srs, but actually fast.**

Vesta is a desktop app for turning video files into translated subtitles and Anki decks. If you've used subs2srs, the workflow will feel immediately familiar — same core idea, rebuilt from scratch to be faster and less painful to use.

Built with Rust (Tauri) + Svelte. It's snappy.

---

## What it does

Load a video. Get subtitles. Translate them. Export an Anki deck with video clips, audio snippets, and screenshot cards — all synced to the exact lines of dialogue. The whole pipeline that used to take an hour now takes a few minutes.

Tested on *Detour* (1945): **~7× faster** than subs2srs end-to-end.

---

## Features

**Transcription** — runs Whisper locally to generate SRT subtitles straight from the audio. No API, no upload.

**Translation** — AI-powered, subtitle-aware. Keeps timing and formatting intact.

**Sync** — an interactive wizard to fix timing mismatches. Set a few manual anchor points, let the auto-sync fill in the rest.

**Flashcards** — generates `.apkg` Anki decks from your subtitles. Each card can include:
- the exact video clip for that line
- an audio snippet
- a screenshot
- surrounding dialogue for context

**Revision** — a built-in SRT editor for when you want to clean things up by hand.

---

## Pipeline

You don't have to start from scratch. Jump in at whatever step makes sense:

```
Video → [Transcribe] → [Sync] → [Translate] → [Flashcards]
```

Already have an SRT? Skip straight to Sync or Flashcards.

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