# Stability audit — 2026-10-01

## Recommendation and scope

Stability and polishing should lead this campaign. A faster export with lost
settings, misleading progress, broken model discovery or overwritten episodes is
less useful than predictable behavior. Keep a short correctness benchmark and
measure representative workloads before pursuing extreme optimization.

This pass inspected workspace layering, frontend IPC and provider catalogs,
network adapters, media generation/preparation, archive output, configuration
persistence, model downloads, CI/hooks and the documentation map. It ran the
existing suites, added targeted regressions and exercised synthetic media plus a
small sample of the reported film. This is not a line-by-line proof of all code,
a full installed-platform certification, or authenticated testing of every vendor.
Existing uncommitted workflow/release/sidebar changes were preserved.

## Findings corrected

| Area | Failure or misleading behavior | Change / verification |
|---|---|---|
| Onboarding | Reload could interrupt fire-and-forget writes and leave the force flag or defaults unsaved | Serialized frontend writes and awaited durable snapshot before reload; regression checks a pending write and disk failure |
| Settings | Full import wrote the same temporary file outside the mutation lock; invalid configuration silently became defaults | Lock covers write and replacement; unique synced temporary file; disk errors propagate; in-memory mutation follows successful persistence |
| OpenAI translation | No explicit default; provider fell through to local Ollama URL | Explicit remote default and regression assertion |
| OpenRouter discovery | `/api/v1` normalization removed a required provider prefix | Repair limited to loopback LM Studio URLs; URL regression tests |
| Provider catalog | Public catalogs no longer list Claude 3.5 Haiku or the three NVIDIA suggestions | Updated to identifiers observed in each provider's public `/models` response; availability still depends on account |
| Cloud STT | GPT-4o models received Whisper-only verbose/timestamp parameters | JSON request without Whisper timestamp parameters; WAV duration supplies coarse whole-chunk timing; unsupported translation rejected |
| Cloud cancellation | Cancellation checked only between chunks; AssemblyAI polling could continue indefinitely | Select over in-flight request and cancellation; 15-minute poll deadline |
| Unicode errors | Truncating response bytes at arbitrary offsets could panic on CJK/emoji | UTF-8-boundary-safe truncation plus regression |
| Media ranges | Empty/invalid/reversed/out-of-bounds ranges could underflow; suffix ranges were misread | Validated parser; 416 with correct size; tests for normal/open/suffix and invalid ranges |
| HTTP adapter | IPv6 loopback host brackets were not parsed; redirects escaped host policy | Bracket-aware IP checks and policy checked on each redirect |
| Gemini credentials | Keys appeared in query URLs despite an authentication header | Header-only API authentication for translation and model discovery |
| Snapshot preparation | Entire films could be transcoded before any snapshots; VA-API availability encouraged unnecessary conversion | Audio/snapshots now use original media; full-film preparation reserved for video clips |
| Preparation progress | UI stayed at 12% through full conversion, and cancellation did not interrupt a status wait | FFmpeg timestamp progress, phase bar, killed child on cancellation, bounded stall detection; integration test |
| Series APKG | Frontend invoked a nonexistent merger and single-mode episodes reused one output path | Registered merger, distinct episode directories, remapped note/card IDs, conflict checks and atomic final archive; two-export regression |
| Embedded subtitles | Only external subtitles were usable in the GUI | Extract tab before Experimental; FFprobe track discovery, atomic SRT extraction, bitmap/OCR distinction |
| Export actions | Footer copied paths and used emoji for media/package counts | Monochrome SVG icons; filename opens via system association; adjacent folder action; visible failure messages |
| API-key navigation | AssemblyAI used an old app path; some URLs redirect to login | Canonical dashboard API-key URL; bounded online inventory, with login/auth limits recorded |
| Documentation | Cloud tier claims, disabled-provider comments, Vulkan build statements and benchmark relevance overstated behavior | Updated root/app/library/module/architecture guides; historical benchmark scope identified |

## Validation results

- 122 frontend tests pass; Svelte/TypeScript reports zero errors and zero warnings.
- 171 Rust tests pass across the complete workspace; no ignored tests.
- Production frontend build, Cargo formatting, strict workspace Clippy and internal version checks pass.
- Three synthetic smoke exports pass archive/media/SQLite checks.
- The Extract tab was visually checked with simulated native IPC; this does not certify native dialogs or Anki associations.
- i18n key/placeholder checks pass. Heuristic localization warnings remain, including English fallback strings outside Italian/English.

## Endpoint evidence

The online inventory found 26 distinct key/documentation/discovery URLs. All
responded with success, login redirects, or access-required responses on this
machine. Mistral's key URL reaches authentication with a 307; this is not proof
of the final page after login. Groq, Mistral and OpenAI model endpoints return
401 without credentials; Gemini returns 403. NVIDIA and OpenRouter model catalogs
are readable without credentials and were used to check their static suggestions.

| Provider | Request contract reviewed/tested | Live limit |
|---|---|---|
| Google Gemini | Native `generateContent`, key header, generation-capable model filtering | Authenticated inference/model availability not tested |
| OpenAI | OpenAI-compatible chat default; multipart STT JSON/Whisper split, bearer auth | Authenticated inference not tested; GPT timing remains coarse |
| Groq | Compatible chat default; transcription/translation multipart and model fallback | Authenticated inference not tested |
| OpenRouter | `/api/v1/models` preserved; public model suggestions checked | Chat inference/account quotas not tested |
| Mistral | Compatible base URL and model route; key login destination | Authenticated chat not tested |
| NVIDIA NIM | Public model route and static suggestions checked | Authenticated chat not tested |
| Deepgram | `/listen`, Token auth, language parameter and utterance conversion in loopback test | Real audio upload/inference not tested |
| AssemblyAI | Upload/create/poll contract in loopback test, bounded polling | Real job/model/language availability not tested |
| Ollama/LM Studio/custom | Loopback/OpenAI URL normalization and custom-host adapter | Actual local server not required by this test suite |
| AnkiConnect | Existing protocol/serialization tests; 15-second request timeout | Actual Anki import/playback still needs installed-app smoke test |
| Media/IPC | Every literal frontend command registered; range parser tests | Real webview range/playback and argument contracts need integration tests |

Authoritative provider references:
[OpenAI transcription and timestamps](https://developers.openai.com/api/docs/guides/speech-to-text),
[AssemblyAI key dashboard](https://www.assemblyai.com/docs/coding-agent-prompts),
[Mistral API keys](https://docs.mistral.ai/admin/identity-access/api-keys),
[GitHub Models retirement](https://github.blog/changelog/2026-07-01-github-models-is-being-fully-retired-on-july-30-2026/),
[OpenRouter catalog](https://openrouter.ai/api/v1/models),
[NVIDIA catalog](https://integrate.api.nvidia.com/v1/models).
GitHub Models remains disabled: the provider retired on 2026-07-30. Historical
configuration may still name it and requires migration to a working provider.

## Reported film and benchmark

The reported *Rear Window* MKV contains HEVC 1792×1080 video, SRT tracks including
English and Italian, and PGS tracks. Both text tracks were extracted locally.
Three subtitle entries generated an APKG with three audio clips and three snapshots
in about one second, without full-film preparation. This verifies a small real
sample; the entire film/deck was not regenerated or imported into Anki here.

The short synthetic debug benchmark on Linux, 16 logical CPUs, two FFmpeg workers,
three samples generated ten notes and twenty media files per run. Median preview
was about 0.004 seconds and median media/APKG export about 0.605 seconds. These
are local baselines, not cross-machine thresholds or evidence that heavy HEVC and
GPU preparation have the same timings. See [QUALITY.md](QUALITY.md).

## Remaining distribution risks and next work

1. Run the installed-bundle matrix on each supported platform. Native file
   associations, GStreamer playback, installers and real GPU fallback were not
   certified by local unit tests or a browser preview.
2. Add opt-in authenticated provider smoke tests with small speech/text fixtures,
   bounded retries and explicit costs. Model existence, a 401 and a correct mocked
   request are three different levels of evidence.
3. Harden persistence UI further: ordinary background settings errors are logged;
   setup/import await failures, but all settings screens should surface disk errors.
   API keys remain plaintext JSON rather than an OS keychain. Unique temporary
   files use restrictive Unix permissions, but backups/exported settings need care.
4. Model downloads still check cancellation after a stream chunk arrives and have
   no read-idle timeout. Add stalled-download tests, checksums and concurrent-download
   ownership before treating model installation as fully robust.
5. The media service uses a session token and loopback binding, but the asset scope
   is broad and CSP is unset. Review the desktop capability/CSP design with an
   actual webview threat model; changing it blindly can break previews.
6. Extend malformed HTTP and cancellation coverage: 429/5xx, malformed success
   JSON, provider failover/resume and AnkiConnect HTTP-status/schema errors.
   Cloud responses without word timing (Deepgram/AssemblyAI text-only fallback)
   need a deliberate timing policy; they currently may have zero-length segments.
7. Improve locale completeness: the new Extract/actions/progress strings are
   localized in Italian/English; other locales initially share English fallback
   values. Existing i18n heuristics also flag pre-existing hardcoded text.
8. Add a representative short HEVC/video-clip benchmark and full-series media
   merge fixture. Existing merge test checks notes/GUIDs/card references; manual
   Anki playback and large-video memory usage remain useful release checks.

Do not call the app “perfectly stable” based on this pass. The repaired regressions
and repeatable gates substantially improve confidence; the remaining platform
and account-dependent checks are explicit release work.

## Follow-up: language UI and authenticated provider checks (2026-10-01)

- Settings flags now use the shared language-to-country mapping, fixing Arabic,
  Hindi, Japanese and Korean. A regression verifies every referenced flag is bundled.
- Embedded subtitle tracks reuse `languages.ts` aliases/detection, sort by language,
  support search by native name/aliases/title/codec, and display ten tracks per page
  in two columns on wide screens. Extraction remains explicit, one track at a time.
- API-key dialog has a stable viewport-bounded height and a scrolling body with
  more space below the provider links; changing providers no longer resizes it.
- Authenticated checks used `.env` credentials without writing them to reports:
  OpenRouter `openrouter/free` completed a tiny prompt; Gemini listed 50 models
  (no inference sent); Groq listed 11 models and completed a tiny GPT-OSS-20B prompt.
  The Groq catalog no longer contained the suggested Llama 3.3/3.1 models, so the
  suggestions were updated and the backend default switched to GPT-OSS-20B.
- Deepgram Nova-3 accepted a 2.22-second locally synthesized WAV, returned a
  nonempty transcript and six timed words. This checks the service response, not
  a full movie transcription or all language/model combinations. The short audio
  can consume provider credits according to its billing minimum.
- Initial Python-default-user-agent Groq request returned a non-JSON 403;
  a Vesta user agent succeeded. No change to the native adapter was needed.
- Follow-up frontend suite: 126 tests pass; Svelte check has no errors or warnings.
  Translation library tests and frontend production build pass.
- Alternatives to local Whisper already include Deepgram Nova-3/Nova-2 and
  AssemblyAI Universal. Flux is intentionally absent from the batch catalog:
  it requires a separate streaming WebSocket protocol at `/v2/listen` and cannot
  be selected interchangeably on the existing `/v1/listen` batch adapter.
  See [Deepgram Flux quickstart](https://developers.deepgram.com/docs/flux/quickstart).

## Follow-up: extraction polish, development and release gates (2026-10-01)

- Pagination moved above the subtitle list beside search, with directional icons.
  File selection uses an add action; downloads use labeled, accessible icon buttons.
  Hover/focus highlights the corresponding row. Bitmap download buttons remain
  visibly disabled with an OCR explanation; the redundant footer note was removed.
- Browser smoke verification used the real Svelte extraction component with mocked
  native dialogs/tracks: ten entries per page, next-page navigation, Italian alias
  search (`giapponese`) and reset to page one were verified. Native extraction is
  covered separately by FFmpeg integration tests, not by this browser mock.
- Cold virtual CSS requests are prepared before the Svelte loader runs, avoiding
  its cache-miss warning and preserving scoped CSS. `npm run test:dev-css` checks
  the three affected components before JS is requested, repeat requests, and a
  fresh server instance. It is now included in the quality gate and CI/release.
- Subtitle language detection, normalization and sorting now run once per loaded
  track list/locale, not on every search keystroke. Search filters a prepared index;
  language alias terms are cached. No timing threshold is asserted for this change.
- The launcher no longer recursively scans Rust build output for an obsolete
  path marker or deletes the entire target directory when it finds one. It uses
  lockfile installation (`npm ci`) and re-executes Bash when called with `sh`.
- Frontend dev invocation no longer uses Unix-only inline environment assignment,
  improving Windows compatibility. Node 22 is used in Vesta CI/build/release, with
  an explicit >=22.12 frontend requirement. Existing dependencies were not upgraded.
- Semantic release now runs frontend type checks, unit tests, cold CSS checks and
  a production build before publishing. CI also builds the frontend. Workflow YAML
  was parsed locally; actual hosted runners and installers still require verification.
- Validation: 127 frontend tests and 171 Rust tests pass; production build, strict
  Clippy, formatting, version checks, synthetic APKG export and dev CSS regression
  pass. i18n still reports 25 nonblocking heuristic warnings. Installer signing,
  Windows/macOS runtime smoke tests, GPU variants and real Anki import remain
  release checklist items; passing this gate is not a distribution certification.

## Follow-up: shared media picker and scoped refactoring (2026-10-01)

- Extraction reuses `PathPickerField` for the media path, Browse and Clear actions;
  the subtitle below the heading was removed. File dialogs use the shared dialog
  guard, preventing overlapping native dialogs across tabs.
- Suggested SRT filenames follow `Movie_name_it.srt`: whitespace becomes
  underscores, language tags/aliases resolve to two-letter codes, unknown language
  uses `xx`, and Windows-invalid filename characters are sanitized. Regional
  variants use the base language. Multiple tracks in one language intentionally
  suggest the same name; users can change it in the save dialog.
- Sidebar switches share a presentational `SidebarToggle`, including accessible
  switch state, activity and disabled state. Both labels use two lines (first word
  above, remaining words below), including Killswitch / AI.
- Flashcards per-episode override groups/comparison/diff moved into the pure
  `episodeMediaSettings` utility. Regression tests cover automatic audio selection,
  explicit track changes, unchanged settings and media overrides. Deck configuration
  and generation behavior are preserved; no broad rewrite of async orchestration
  was attempted.
- Complete quality gate passes: 131 frontend tests, 171 Rust tests, cold CSS checks,
  production build, version checks and synthetic APKG export. Previously recorded
  distribution/platform checks remain outstanding. The large Flashcards,
  Translate, Settings and Transcribe tabs remain candidates for future incremental
  separation of orchestration and presentation; their size alone is not evidence
  of a runtime performance problem.

## Follow-up: preloaded extraction UI and language metadata (2026-10-01)

- The small extraction component is loaded with the app and mounted once; larger
  feature tabs retain their lazy loading. Mounting extraction does not probe media
  or open dialogs. Search, pagination and ten neutral track placeholders are
  present immediately; a reserved status area avoids shifting the toolbar during
  probing. Controls become enabled when tracks arrive. The search field includes
  a magnifying-glass icon. Loading animation respects reduced-motion preferences.
- Track presentation moved into `SubtitleTrackCard`; extraction/dialog state stays
  in its tab. Browser smoke checks with delayed mocked native responses covered the
  initial shell, loading, populated results and Clear returning to the same shell.
- The actual Rear Window MKV labels streams 30 (Korean title) and 37 (Turkish title)
  with the English `eng` language tag. Vesta now exposes a title/tag disagreement
  rather than silently assuming the title describes the real subtitle content.
  Metadata remains authoritative until users inspect/correct the source.
- Language detection now resolves exact codes/names/aliases via a precomputed map,
  preserving first-match behavior for shared ISO aliases. Fuzzy matching remains
  available for descriptive text. Tests cover aliases, regional variants and
  conflicting tags/titles. This reduces repeated fuzzy work without adding caches
  of arbitrary user text or a network dependency.
- Validation: 133 frontend tests, 171 Rust tests, frontend production build, cold
  CSS/restart checks, version consistency and synthetic APKG export pass. Installer
  signing, platform/GPU smoke tests and real Anki import remain outstanding as
  documented above. No release was published during this audit.

## Follow-up: navigation latency and complete localization (2026-10-01)

- Feature tabs are warmed sequentially in idle slots after startup, with additional
  preload on pointer hover/keyboard focus. Loaded tab instances stay mounted, so
  later navigation does not reload a chunk or reconstruct the screen. Teardown
  cancels pending warm-up work. Unit tests cover sequential loading, failures and
  cancellation. Initial cold imports can still take time before warm-up completes.
- Subtitle pagination reuses its ten component slots instead of destroying and
  recreating every row. A browser check on the real extraction component with mock
  media/dialog IPC measured next-frame update at 3.1–3.6 ms, confirmed reuse of all
  rows and no additional probing on page changes. This is a browser measurement,
  not a claim about all GPUs/WebKit configurations or cold startup time.
- All 15 catalogs now contain 1,118 keys. Localization was centralized for setup,
  settings macro copy, backup/restore, shortcut search, file-drop hints, navigation
  fallback messages, accessibility labels and replacement confirmations. English
  and Italian inline branches were removed. The wizard uses the selected language
  consistently while its dictionary loads. Provider descriptions use translated
  catalog text; Groq's retired model names were removed from its descriptions.
- 1,738 previously missing/fallback entries were drafted using explicitly free
  OpenRouter models (reported cost zero), then structurally validated and sampled.
  A Japanese response in Chinese was rejected during review; all 140 affected
  Japanese entries were manually replaced. New language names use Intl.DisplayNames
  in the active UI language, keeping ISO aliases/native names searchable. Native
  linguistic review can still improve wording; this audit verifies coverage and
  parameters, not professional translation certification.
- Catalog tests enforce all keys/nonempty strings and exact interpolation tokens
  in every locale, and check representative new prose against English fallback.
  The source scanner now checks setup and settings wrapper calls as well as `t`.
  CI/release/local quality gate block same-as-English leaks. Exceptions are scoped
  to genuine shared words, product names and parameter-only messages.
- Validation: 139 frontend tests, 171 Rust tests, production build, cold CSS/restart
  checks, full localization audit, internal versions and APKG smoke pass. The source
  hygiene scan reports zero errors and six technical-name heuristic warnings.
  Remaining installer/platform/GPU/Anki release checks are unchanged.

## Frontend maintenance pass — 2026-10-02

Separated flashcard series/single generation, package aggregation/merging and
bounded companion-file discovery from `FlashcardsTab`. Transcription endpoint
readiness, credential resolution, failover and backend/model/VAD discovery are
owned by focused modules in `src/lib/workflows`; the live transcript is a
presentation-only panel. README and architecture boundaries were updated.

Regression coverage now exercises cancelled requests (including late success),
failover on unsuccessful responses, merge failure and partial episode success,
run output-setting snapshots, out-of-order/disposed model discovery, custom VAD
availability and bounded deterministic file discovery. Transcribe keyboard
listeners are removed on unmount and ignored while the tab is inactive. A new
attempt clears previous attempt segments/progress; reset clears live segments.

Validation: 160 frontend tests passed, Svelte check reported no errors or warnings,
production build and cold/repeated/restarted development CSS checks passed.
The localization source check has only the six existing technical-name warnings.
These headless checks do not replace an installed desktop/Anki smoke test.

## Subtitle grouping and preview — 2026-10-02

Grouped variants by language, with downloadable languages first, explicit title
language resolving contradictory tags (still visibly flagged), and ordinary text
selected before special/bitmap variants. Removed the empty status-row gap.
Added an eye action, double click and context menu opening a bounded plain-text
preview dialog, with temporary extraction, 100-cue/64-KiB caps and a 16-entry cache.

Verified selectors, pagination, context menu and double click in the browser with
real film stream metadata and mocked preview IPC. Real FFmpeg preview of Rear
Window produced 100 cues; synthetic Rust tests cover extraction/preview and invalid
track failure. Full quality gate, desktop tests, synthetic APKG smoke, frontend
localization/build and strict Clippy passed locally before publication.
