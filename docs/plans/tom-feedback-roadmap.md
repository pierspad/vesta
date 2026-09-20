# Piano di lavoro — feedback di Tom (11 ago 2026)

Documento operativo per un agente esecutore. Ogni task elenca: file toccati, API
da introdurre, criteri di accettazione e test. Le decisioni aperte sono marcate
**[DECISIONE]** e vanno riportate all'utente prima di procedere, non risolte
d'iniziativa.

Vincoli di progetto (da `docs/ARCHITECTURE.md` e `.cursorrules` §9):

* La logica sta nelle crate `lib/`, **senza accoppiamento a Tauri**: progress via
  callback, cancellazione via `CancellationToken`.
* Ogni feature engine ha una CLI headless sottile in `cli/`.
* `apps/srt-gui/src-tauri` contiene solo wrapper di comando, zero business logic.
* Le versioni delle crate interne sono in lock-step
  (`build-scripts/check_internal_crate_versions.sh`).
* Iterare con `cargo check -p <crate>` sulle lib, non sull'app intera.

---

## 0. Stato reale rispetto al feedback

Verificato sul codice a `c0395d7`. Non ripartire da assunzioni diverse.

| Richiesta di Tom | Stato | Riferimento |
|---|---|---|
| Compressione media (WebP/AVIF + Opus) | **Fatto** | `lib/srt-flashcards/src/types.rs` (`SnapshotFormat`, `AudioFormat`) |
| Audio sul fronte della carta | **Fatto** | template note type in `apps/srt-gui/src/lib/types/noteTypes.ts` |
| Preset di qualità disaccoppiati snapshot/video | **Fatto** | `SnapshotsPanel.svelte`, `VideoClipsPanel.svelte` |
| Preset di **risoluzione** snapshot (144p/240p/360p/480p) | **Fatto** | `RESOLUTION_PRESETS` in `apps/srt-gui/src/lib/panels/SnapshotsPanel.svelte:37` |
| Drag & drop dei file, anche per i film | **Fatto** | `apps/srt-gui/src/lib/utils/dragDrop.ts` + `FlashcardsTab.svelte:963` (`handleFileDrop`, ramo `else` = modalità film) |
| Font CJK/internazionali nel CSS Anki | **Da fare** | Workstream A |
| Tagging automatico di difficoltà (CEFR/HSK/JLPT) | **Da fare** | Workstream B |
| Modelli STT leggeri alternativi a Whisper | **Da fare** | Workstream C |
| OCR di sottotitoli hardcoded (pag. 9 del PDF) | Backlog, fuori scope | — |

Nota su pag. 6 del PDF: il file `..._0031.mp3.opus` con doppia estensione viene
dalla conversione manuale di Tom con ffmpeg su un mazzo già esistente, **non** è
un bug di Vesta: i nomi passano tutti da `media_filename()`
(`lib/srt-flashcards/src/media.rs:31`), che usa una sola estensione derivata dal
formato.

**Prima attività, prima di scrivere codice:** una verifica manuale di 10 minuti
del drag & drop in modalità *Film* (trascinare un video + due `.srt` sulla
finestra) e dei preset di risoluzione. Se funzionano, l'unica azione residua è di
*scopribilità*: il placeholder del riquadro dice "Nessun file aggiunto, inizia
aggiungendo i sottotitoli originali" e non menziona il trascinamento. Aggiungere
al placeholder una riga tipo "…oppure trascina qui i file" (chiave i18n nuova,
vedi §D per la procedura sulle 15 locale). Task da ~30 minuti, non un workstream.

**Ordine consigliato:** A → B → C. A è piccolo, a rischio zero e sblocca subito
la leggibilità sui dispositivi di Tom; B è il pezzo ad alto impatto; C è un
progetto a sé e va aperto solo dopo uno spike.

---

## Workstream A — Font per lingua nel CSS Anki

Obiettivo: le carte usano una pila di font adatta alla lingua studiata, così su
Android/Linux non compaiono i *tofu* (rettangoli vuoti) né font di ripiego
illeggibili.

Scelta di fondo: **fase A1 = solo pila di font (`font-family`)**, nessun font
incorporato. Il font embedding nel `.apkg` (fase A2) è realistico ma cambia
categoria di problema — vedi in fondo.

### A1.1 — Mappa lingua → pila di font (Rust)

Nuovo file `lib/srt-flashcards/src/fonts.rs`, esportato da `lib.rs`.

```rust
/// Pila CSS `font-family` per il codice lingua dei sottotitoli target.
/// `lang` è un codice come quelli di apps/srt-gui/src/lib/config/languages.ts
/// ("zh", "zh-tw", "ja", "ko", "ar", "he", "th", "hi", "el", "ru", "pt-br", …).
/// Confronto case-insensitive; sconosciuto → `DEFAULT_FONT_STACK`.
pub fn font_stack_for(lang: &str) -> &'static str;

pub const DEFAULT_FONT_STACK: &str = "…";
```

Regole per la mappa (ogni voce: Noto specifico → Noto generico → font di sistema
per piattaforma → `sans-serif`):

| Lingua | Pila (in ordine) |
|---|---|
| `ja` | `Noto Serif CJK JP`, `Noto Sans CJK JP`, `Noto Sans JP`, `Hiragino Sans`, `Yu Gothic`, `Meiryo`, `sans-serif` |
| `zh` | `Noto Sans CJK SC`, `Noto Sans SC`, `Source Han Sans SC`, `PingFang SC`, `Microsoft YaHei`, `sans-serif` |
| `zh-tw` | `Noto Sans CJK TC`, `Noto Sans TC`, `PingFang TC`, `Microsoft JhengHei`, `sans-serif` |
| `ko` | `Noto Sans CJK KR`, `Noto Sans KR`, `Apple SD Gothic Neo`, `Malgun Gothic`, `sans-serif` |
| `ar` | `Noto Naskh Arabic`, `Noto Sans Arabic`, `Geeza Pro`, `Segoe UI`, `sans-serif` |
| `he` | `Noto Sans Hebrew`, `Arial Hebrew`, `Segoe UI`, `sans-serif` |
| `th` | `Noto Sans Thai`, `Thonburi`, `Leelawadee UI`, `sans-serif` |
| `hi` | `Noto Sans Devanagari`, `Kohinoor Devanagari`, `Nirmala UI`, `sans-serif` |
| `el`, `ru`, `uk` | `Noto Sans`, `Segoe UI`, `Helvetica Neue`, `sans-serif` (coprono greco/cirillico) |
| altro | `DEFAULT_FONT_STACK` = `Noto Sans`, `-apple-system`, `Segoe UI`, `Roboto`, `Helvetica Neue`, `Arial`, `sans-serif` |

Dettagli che l'esecutore deve rispettare:

* I nomi con spazi vanno tra virgolette **doppie** nel CSS; il valore viene poi
  serializzato dentro una stringa JSON del modello Anki
  (`export_apkg.rs:243` usa `serde_json::to_string`), quindi l'escaping è già
  gestito: non aggiungere escaping a mano.
* `zh-tw` va risolto **prima** di `zh` (match esatto sul codice completo, poi
  fallback sul prefisso prima del `-`).

### A1.2 — Passare la lingua alla config

`lib/srt-flashcards/src/types.rs`, in `FlashcardConfig`:

```rust
/// Codice lingua dei sottotitoli target; guida la scelta del font della carta.
#[serde(default)]
pub target_language: Option<String>,
/// Se false, Vesta non inietta la variabile del font nel CSS del modello.
#[serde(default = "default_true")]
pub auto_card_font: bool,
```

Aggiornare `FlashcardConfig::default()`/costruttori e il JSON di esempio nei test
in fondo a `types.rs` (c'è già un literal di config completo lì).

### A1.3 — Iniezione nel CSS del modello

In `lib/srt-flashcards/src/export_apkg.rs`, dove oggi si fa
`let css = config.card_css.as_deref().unwrap_or(ANKI_CARD_STYLING);` (riga ~237):

```rust
let base_css = config.card_css.as_deref().unwrap_or(ANKI_CARD_STYLING);
let css = maybe_prepend_font_vars(base_css, config);   // nuova fn in fonts.rs
```

`maybe_prepend_font_vars` antepone, solo se `auto_card_font`:

```css
/* vesta:font-start */
:root { --vesta-target-font: <stack>; }
.card { font-family: var(--vesta-target-font); }
/* vesta:font-end */
```

Requisiti:

* **Idempotenza**: se il CSS in ingresso contiene già `/* vesta:font-start */`,
  sostituire il blocco esistente invece di accodarne un secondo (l'utente può
  salvare nel proprio template un CSS già passato da qui).
* Il blocco va **in testa**, così un `font-family` scritto a mano dall'utente più
  in basso continua a vincere per specificità/ordine: l'automatismo non deve
  sovrascrivere una personalizzazione esplicita.

Aggiornare anche `ANKI_CARD_STYLING` (`export_apkg.rs:579+`) e il CSS di default
del GUI (`apps/srt-gui/src/lib/types/noteTypes.ts:351`), che sono due copie da
tenere allineate: sostituire `font-family: arial;` con
`font-family: var(--vesta-target-font, arial);` in `.card`, `.reading`,
`.meaning`, `.expression`. **Non** toccare `.tag-pill` (deve restare latino).

### A1.4 — GUI

* `apps/srt-gui/src/lib/utils/flashcardConfig.ts`: aggiungere `target_language`
  e `auto_card_font` al payload. La lingua studiata è già disponibile nel tab
  (`getStudiedLanguagePreference()` in `FlashcardsTab.svelte:192`); passarla
  tramite `FlashcardConfigInputs` (nuovo campo `targetLanguage: string | null`),
  valorizzata sia nel ramo film sia nel ramo serie.
* `apps/srt-gui/src/lib/panels/AnkiSettingsPanel.svelte`: un `ToggleRow`
  "Font automatico per la lingua" (default **on**) + testo di aiuto: il font
  viene *richiesto*, non incorporato; se manca sul dispositivo Anki ripiega su
  quello successivo della lista.
* Persistenza con il pattern già in uso (`vestaConfig` / store dedicato), non
  `localStorage` diretto.

### A1.5 — Test

* `lib/srt-flashcards/src/fonts.rs`: unit test su `font_stack_for` — `zh-tw` ≠
  `zh`, case-insensitive, sconosciuto → default.
* Test di `maybe_prepend_font_vars`: iniezione, idempotenza (due passaggi = un
  solo blocco), `auto_card_font: false` → CSS invariato byte per byte.
* Aggiornare i test esistenti dell'esportatore apkg in
  `lib/srt-flashcards/tests/` se confrontano il CSS.

### Accettazione A1

Generare un `.apkg` con lingua target `zh`, importarlo in Anki, aprire il modello
di carta: il CSS contiene il blocco `vesta:font-start` con la pila cinese e le
carte usano quel font. Con il toggle disattivato il CSS torna identico a prima.

### A2 (opzionale, decidere dopo) — font incorporato nel deck

Solo se A1 non basta sul tablet Android di Tom. Anki carica i font dalla cartella
media se il nome inizia con `_` (i file `_*` sono esclusi dal "check media").
Serve: scaricare il subset Noto della lingua, salvarlo nel pacchetto come
`_vesta-<lang>.woff2`, emettere una `@font-face` nel CSS e includere la licenza.

**[DECISIONE]** Costo reale: un Noto CJK anche sottoinsiemato pesa svariati MB e
finisce in *ogni* mazzo — in diretto conflitto con il punto "peso dei mazzi" dello
stesso feedback. Va proposto come opzione esplicita e disattivata di default, mai
come automatismo. I font Noto sono sotto SIL OFL: la ridistribuzione è permessa
ma **richiede** di allegare il testo della licenza nel pacchetto.

---

## Workstream B — Tagging automatico di difficoltà (CEFR / HSK / JLPT)

Obiettivo: ogni nota esportata riceve un tag Anki reale con il livello della
parola più difficile della frase (`HSK::3`, `CEFR::B1`, …), così lo studente
filtra il mazzo in Anki.

Punto di partenza importante: **oggi la colonna `tags` delle note è sempre
vuota** (`export_apkg.rs:395`, letterale `''` in fondo alla INSERT). Il campo
"Tags" del note type è un'altra cosa: contiene `<deck>_<ep>`
(`export_apkg.rs:361`). Il template di default mostra già `{{Tags}}` come pillole
(`#tags-container` + `tags-source`), quindi appena si popola la colonna i tag
compaiono anche sulla carta senza toccare l'HTML.

### B0 — [DECISIONE] Fonti dati e licenze

Da risolvere **prima** di scrivere il motore: l'esecutore non deve scegliere da
solo cosa incorporare nel binario.

| Schema | Candidato | Da verificare |
|---|---|---|
| HSK | liste ufficiali HSK 3.0 (2021), ~11.000 parole su 9 livelli | licenza del repo scelto; se mappare i livelli 7-9 su un unico `HSK::7-9` |
| JLPT | liste N5-N1 non ufficiali derivate da JMdict | JMdict è CC BY-SA 4.0: obbliga ad attribuzione e condivisione allo stesso modo del file derivato |
| CEFR (inglese) | English Vocabulary Profile **non** è liberamente ridistribuibile; alternative: CEFR-J Wordlist (CC BY-SA) o bande di frequenza | licenza e qualità |
| CEFR (altre lingue) | nessuna lista autorevole libera: usare bande di frequenza da `hermitdave/FrequencyWords` (OpenSubtitles, MIT) mappate su A1…C2 per rango | accettabilità dell'approssimazione |

Nota onesta da riportare all'utente: per le lingue senza lista ufficiale il tag è
una **stima basata sulla frequenza**, non un livello certificato. L'etichetta in
GUI deve dirlo (es. `CEFR≈` o una nota accanto al selettore), altrimenti si
promette una precisione che non c'è.

Formato dati adottato: un file per pacchetto, TSV `parola<TAB>livello`, ordinato,
UTF-8, compresso, incorporato con `include_bytes!` e decompresso una volta sola
in una `OnceLock<HashMap<…>>`. Dimensioni attese: 100-300 KB per pacchetto
compresso — accettabile nel binario. Deve esistere anche il caricamento di un
file utente esterno (stesso formato) per chi vuole la propria lista.

### B1 — Crate `lib/srt-difficulty` [RIMOSSA]
*(Nota: funzionalità rimossa su richiesta per evitare complessità eccessiva e sovradimensionamento dell'applicazione).*

Crate pura, nessuna dipendenza da Tauri o ffmpeg. Da aggiungere ai membri del
workspace (già coperti da `lib/*`) e alle `workspace.dependencies` con la stessa
versione delle altre crate interne.

API pubblica:

```rust
pub enum LevelScheme { Cefr, Hsk, Jlpt }

pub struct LevelTable { /* parola -> livello, lunghezza max token */ }
impl LevelTable {
    pub fn builtin(scheme: LevelScheme, lang: &str) -> Result<&'static LevelTable>;
    pub fn from_tsv(input: &str) -> Result<LevelTable>;
}

pub struct AnalyzeOptions {
    /// Parole non presenti in tabella: ignorate, oppure trattate come livello massimo.
    pub unknown: UnknownPolicy,
    /// Token più corti di così vengono ignorati (rumore, particelle).
    pub min_token_chars: usize,
}

pub struct CardLevel {
    pub level: Option<u8>,      // None = nessuna parola riconosciuta
    pub known_tokens: usize,
    pub unknown_tokens: usize,
}

pub fn analyze(text: &str, table: &LevelTable, opts: &AnalyzeOptions) -> CardLevel;

/// "HSK::3", "CEFR::B1", "JLPT::N3" — N.B. JLPT è invertito (N5 = più facile).
pub fn tag_for(scheme: LevelScheme, level: u8) -> String;
```

Tokenizzazione — punto tecnico chiave, evita dipendenze pesanti:

* **Scritture senza spazi (zh, ja)**: segmentazione *longest-match* sulla tabella
  stessa (finestra massima = parola più lunga della tabella, tipicamente 4). È
  quello che fanno di fatto gli strumenti HSK e non richiede né `jieba-rs` né
  `lindera` (che si porterebbe dietro un dizionario da centinaia di MB). Il
  carattere non abbinato viene saltato di uno e conta come token sconosciuto.
* **Scritture alfabetiche**: split su non-alfanumerici Unicode, lowercase, e
  fallback banale di lemmatizzazione (prova la forma piena, poi senza
  `s`/`es`/`ed`/`ing` per l'inglese). Niente stemmer esterni.
* Scartare sempre: cifre, punteggiatura, testo latino quando lo schema è CJK,
  tag HTML residui (`<br>`, `<span …>` — il testo arriva già renderizzato in
  alcuni percorsi: normalizzare prima).

Il livello della carta = **massimo** fra i livelli dei token riconosciuti (è
esattamente ciò che chiede Tom: "highest vocabulary level per card").

### B2 — Integrazione in `srt-flashcards`

* `types.rs`: nuovo blocco opzionale in `FlashcardConfig`

  ```rust
  #[serde(default)]
  pub difficulty: Option<DifficultyConfig>,
  // { scheme, language, unknown_policy, tag_prefix: Option<String> }
  ```

* `lib.rs`: nuovo stadio dopo `build_matched_lines()` e prima dell'export, che
  calcola un `Vec<Option<String>>` di tag allineato alle righe attive. Emettere
  un `FlashcardProgressEvent` con `stage: "difficulty"` per coerenza con gli
  altri stadi (l'analisi è veloce, ma la barra non deve fare salti muti).
* `export_apkg.rs`: scrivere i tag nella colonna `tags` della INSERT. Convenzione
  Anki: stringa **delimitata da spazi anche in testa e in coda**, es.
  `' HSK::3 '`. Applicare l'escaping SQL già usato per gli altri campi
  (`replace('\'', "''")`). Nessun tag → mantenere `''`.
* `export_tsv.rs`: aggiungere una colonna finale con i tag **solo** se il tagging
  è attivo, e documentare nell'help della CLI che in Anki va mappata su "Tags".
  Attenzione: cambiare il numero di colonne rompe i mapping salvati dagli utenti,
  quindi la colonna è condizionale, non incondizionata.
* **Filtro** (fase 2, non nel primo giro): `min_level` / `max_level` in
  `SubtitleFilters` per scartare le carte fuori range già in generazione. Tom
  preferisce filtrare in Anki, quindi non è bloccante.

### B3 — CLI

`cli/srt-flashcards-cli`: flag `--difficulty <cefr|hsk|jlpt>`,
`--difficulty-language <code>`, `--difficulty-wordlist <path>` (lista custom),
`--difficulty-unknown <ignore|highest>`. Restano flag opzionali: senza di essi il
comportamento è identico a oggi.

### B4 — GUI

* Nuovo pannello `apps/srt-gui/src/lib/panels/DifficultyPanel.svelte`, montato
  nel `FlashcardsTab` accanto a "Filtri flashcard": toggle + select dello schema
  (preselezionato dalla lingua studiata: zh → HSK, ja → JLPT, resto → CEFR) +
  select per la politica sulle parole sconosciute + nota sull'approssimazione.
* `flashcardConfig.ts`: serializzare il blocco `difficulty`.
* Anteprima (`FlashcardsPreviewModal`): mostrare la distribuzione dei livelli
  (quante carte per livello) prima di generare. È la funzione che rende davvero
  utile la feature — se il tempo stringe, si può rimandare, ma va tenuta in
  testa alla lista dei follow-up.

### B5 — Test

* `lib/srt-difficulty`: longest-match su una frase cinese con parole di 1/2/3
  caratteri e una parola fuori lista; inglese con plurali e punteggiatura;
  `unknown = Highest` vs `Ignore`; tabella vuota → `level: None`.
* `lib/srt-flashcards`: test golden che apre l'`.apkg` generato e verifica il
  contenuto della colonna `tags` (c'è già `tests/detour_golden.rs` come modello);
  test che con `difficulty: None` l'output è **identico** a prima (protezione
  contro regressioni sui mazzi esistenti).

### Accettazione B

Un `.apkg` cinese importato in Anki mostra i tag `HSK::1`…`HSK::6` nella barra
laterale, `tag:HSK3` filtra le carte giuste, e le pillole compaiono sulla carta
(come nello screenshot di pag. 7 del PDF). Con la feature spenta, nessuna
differenza rispetto a oggi.

---

## Workstream C — Motori STT alternativi a Whisper

Non aprirlo insieme a B. È un progetto a sé: tocca `lib/srt-transcribe`
(oggi interamente costruito su `whisper-rs`: `transcribe.rs`, `pipeline.rs`,
`model.rs`), il packaging Tauri e la CI multi-piattaforma.

### C0 — Spike prima di tutto (timebox 1 giorno)

Verificare su Linux **e** Windows che `sherpa-rs` (binding Rust di sherpa-onnx)
compili nel workspace e trascriva 30 secondi di audio con SenseVoice. Motivo
della scelta: una sola integrazione copre SenseVoice (cinese, ~160 MB),
Moonshine (inglese, 31-192 MB), Parakeet, Paraformer e un VAD, cioè quasi tutta
la lista di Tom. Alternative se lo spike fallisce: `ort` (ONNX Runtime) con
pipeline scritta a mano — molto più lavoro per modello.

Esito dello spike da riportare all'utente con: tempi di build, dimensione delle
librerie native da impacchettare, RTF misurato su CPU. Se il costo di packaging
è alto, la feature va ridiscussa, non forzata.

### C1 — Astrazione del motore

In `lib/srt-transcribe`, trait `SpeechEngine` con: caricamento modello, lingua
supportata, trascrizione di un chunk PCM → segmenti temporizzati, cancellazione.
Portare l'implementazione Whisper esistente dietro il trait **senza cambiarne il
comportamento** (refactor puro, test invariati), poi aggiungere il backend
sherpa-onnx.

### C2 — Registro modelli

Estendere `model.rs`: oggi `WHISPER_MODELS` è una tupla piatta e
`model_file_path()` presuppone i nomi `ggml-*.bin`. Serve un registro con
`engine`, `id`, lingue supportate, dimensione, URL, checksum, e un
`downloaded()` che non assuma il formato ggml. Mantenere la retrocompatibilità
dei modelli già scaricati nella cache utente (`~/.cache/whisper`).

### C3 — GUI

Selettore modelli raggruppato per motore e lingua, con suggerimento del modello
consigliato per lingua studiata e RAM disponibile (`cpuRamStore` esiste già), e
riuso delle tier list (`TranscribeTiers.svelte`).

### C4 — Benchmark

`apps/whisper-bench` va esteso ai nuovi motori: senza numeri comparabili
(RTF, WER approssimato sui `Test_Subs/`) la scelta del modello resta un'opinione.

### [DECISIONE] licenze dei modelli

Da verificare prima di elencarli nel downloader: Parakeet (NVIDIA, CC BY-4.0),
SenseVoice, Moonshine, GigaAM. Vesta scarica i pesi, non li ridistribuisce, ma
l'attribuzione va comunque mostrata nella UI.

---

## D. Regole trasversali per l'esecutore

1. **i18n**: ogni stringa nuova va aggiunta a tutte e 15 le locale in
   `apps/srt-gui/src/lib/i18n/locales/`. Scrivere `en.json` e `it.json` con cura;
   per le altre una traduzione ragionevole, mai la chiave nuda o l'inglese
   silenzioso. Nessuna stringa hardcoded nei `.svelte`.
2. **Retrocompatibilità della config**: tutti i campi nuovi di `FlashcardConfig`
   sono `#[serde(default)]`. Un payload salvato dalla versione precedente deve
   continuare a deserializzare.
3. **Test prima del codice** dove il comportamento è definibile (tokenizzatore,
   mappa dei font, iniezione CSS): sono funzioni pure, il TDD qui costa poco e
   rende la review verificabile.
4. **Verifica prima di dichiarare fatto**: `cargo test -p <crate>`,
   `cargo clippy -p <crate>`, `npm run check` nel GUI, e almeno una generazione
   reale di `.apkg` importata in Anki per A e B. Incollare l'output, non
   riassumerlo.
5. **Un workstream per branch**, commit separati per lib / CLI / GUI / i18n.
6. **Bump di versione**: se si tocca una crate interna, allineare tutte le altre
   (`build-scripts/check_internal_crate_versions.sh` fallisce altrimenti).

## E. Checkpoint di review

Punti in cui l'esecutore si ferma e passa la palla:

* **R0** — dopo la verifica manuale di §0 (drag & drop, preset): conferma di cosa
  resta davvero da fare.
* **R1** — dopo B0: fonti dati scelte e licenze verificate. *Bloccante.*
* **R2** — a fine A1, prima della GUI: revisione dell'iniezione CSS su un `.apkg`
  reale.
* **R3** — a fine B1 (crate `srt-difficulty` con i test verdi), prima di
  integrarla nell'export.
* **R4** — a fine B2/B3, con un `.apkg` importato in Anki e uno screenshot dei
  tag.
* **R5** — esito dello spike C0, prima di qualsiasi altro lavoro sul workstream C.
