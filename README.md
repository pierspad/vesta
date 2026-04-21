# Vesta

**subs2srs, but actually fast.**

Vesta is a desktop app for turning video files into translated subtitles and Anki decks. 
If you've used subs2srs, the workflow will feel immediately familiar; same core idea, rebuilt from scratch to be faster and less painful to use.

Built with Rust (Tauri) + Svelte.

---

## What it does

Load a video. Get subtitles. Translate them. Export an Anki deck with video clips, audio snippets, and screenshot cards all synced to the exact lines of dialogue. The whole pipeline that used to take an hour now takes a few minutes.

Tested on [*Detour* (1945)](https://archive.org/details/detour1945HD): **~7× faster** than subs2srs end-to-end using a 4.3Ghz 8 cores CPU.

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