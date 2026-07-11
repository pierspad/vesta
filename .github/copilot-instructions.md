---
description: Linee guida LLM per VESTA, i18n, qualita e release.
---

# Istruzioni Copilot per VESTA

Queste istruzioni vengono lette automaticamente da GitHub Copilot e da altri LLM compatibili quando lavorano in questo workspace. Devi aderire a queste regole quando scrivi codice, aggiorni documentazione o prepari una release.

## 1. Changelog operativo e Release Notes obbligatori

Ogni volta che completi un task, bugfix, refactor o aggiornamento significativo, devi aggiornare contemporaneamente SIA `docs/list_of_things_changed.md` SIA `docs/release-notes.md`.

- Per `docs/list_of_things_changed.md` (registro tecnico/operativo):
  - Mantieni il titolo principale `# Modifiche recenti (in preparazione alla release)`.
  - Raggruppa le modifiche sotto sezioni `## [Data Odierna] - Categoria`.
  - Usa bullet nel formato `- **Etichetta breve**: Descrizione concreta della modifica`.
  - Scrivi cosa e' cambiato e perche' e' utile all'utente o al progetto.
- Per `docs/release-notes.md` (curated release notes per gli utenti finali):
  - Aggiorna la sezione corrente (sotto `## Release Notes` o la versione attiva).
  - Mantieni le categorie come `### Fixes` e `### Improvements`.
  - Usa bullet nel formato `* **Area**: Descrizione sintetica e concreta della modifica per l'utente finale`.
  - Non creare sezioni vuote.
- Non chiedere conferma prima di aggiornare questi file: aggiornali entrambi in parallelo ad ogni task o bugfix terminato.

## 2. Internazionalizzazione a 15 lingue

VESTA deve restare tradotto nelle stesse 15 lingue di TextMerger:
`ar`, `de`, `en`, `es`, `fr`, `hi`, `it`, `ja`, `ko`, `nl`, `pl`, `pt`, `ru`, `tr`, `zh`.

Quando modifichi testi visibili all'utente:

- non introdurre stringhe UI hardcoded nei componenti Svelte;
- usa sempre `t("chiave.di.traduzione")` e i file in `apps/srt-gui/src/lib/i18n/locales`;
- aggiorna sia i locale principali sia `apps/srt-gui/src/lib/i18n/locales/info` quando tocchi contenuti di aiuto;
- aggiorna tutte le 15 lingue nello stesso commit/task;
- preserva placeholder come `{{count}}`, markup necessario e struttura JSON;
- usa gli script in `apps/srt-gui/scripts` quando devi spezzare, fondere o controllare traduzioni;
- esegui `npm run i18n:audit` dalla cartella `apps/srt-gui/` e correggi chiavi mancanti o vuote.

## 3. Qualita, riuso e debito tecnico

Ogni modifica deve ridurre o almeno non aumentare il debito tecnico.

- Preferisci componenti riutilizzabili a duplicazioni locali di UI o logica.
- Centralizza pattern ripetuti come snackbar, modali, shortcut, form controls, gestione errori e helper i18n.
- Mantieni le funzionalita core di parsing, sync, estrazione e traduzione SRT lato Rust/Tauri quando sono computazionalmente rilevanti.
- Evita refactor larghi non richiesti, ma se tocchi codice duplicato valuta un'estrazione piccola e chiara.
- Aggiungi test o audit proporzionati al rischio della modifica.

## 4. Architettura e stile Svelte

- Frontend: Svelte/Vite in `apps/srt-gui`.
- Backend CLI/Tauri: Rust in `core/`, `lib/`, `cli/` e `apps/srt-gui/src-tauri/`.
- Usa le Runes di Svelte 5 (`$state`, `$derived`, `onclick`) dove il file le usa gia'.
- Mantieni il CSS locale al componente o usa Tailwind; evita global CSS se non serve.
- Leggi i `Cargo.toml` rilevanti prima di toccare crate o dipendenze Rust.

## 5. Release notes e pubblicazione

La GitHub Release deve usare `docs/release-notes.md` come corpo curato della release. Non sostituirla con release notes generate automaticamente.

Formato preferito:

```markdown
## Release Notes

### Fixes

* **Area**: Fix sintetico e concreto

### Improvements

* **Area**: Miglioramento sintetico e concreto
```

Regole:

- usa `*` nelle release notes, non `-`;
- mantieni categorie brevi come `Fixes`, `Improvements`, `Packaging`, `Localization`, se servono;
- non creare sezioni vuote;
- scrivi per utenti finali: il dettaglio operativo resta in `docs/list_of_things_changed.md`.

Il rilascio è interamente gestito dalla GitHub Action `release.yml` (nessuno script locale — `git-release.sh`/`push-aur.sh`/`build-aur.sh` sono pattern ritirati):
- **Si attiva da sola su ogni push su `main`**, leggendo i Conventional Commits accumulati dall'ultimo tag: `feat:` → minor bump, `fix:`/`perf:` → patch bump, `feat!:` o un footer `BREAKING CHANGE:` → major bump. Se non c'è nessun commit `feat`/`fix`/`perf`/breaking (solo `chore`/`docs`/`refactor`/`test`/`ci`/`build`) il rilascio viene saltato in automatico, senza errori.
- L'intestazione principale `## Release Notes` viene riallineata automaticamente in `## Release Notes vX.Y.Z` in base alla versione bumpata nel PKGBUILD.
- Se `docs/release-notes.md` non è stato toccato dall'ultimo tag, la Action ne rigenera il contenuto dai messaggi di commit invece di pubblicare note stantie — per questo conviene comunque scrivere i bullet a mano durante lo sviluppo (§1), altrimenti la release finale mostra solo i commit grezzi. `docs/list_of_things_changed.md` non viene toccato dalla pipeline: resta un log tecnico manuale, va aggiornato a mano come da §1.
- Rimane disponibile anche il trigger manuale: Actions → "Release (semver bump)" → Run workflow, con bump `auto|patch|minor|major` o una versione esplicita.

## 6. Comandi utili

- Frontend: `cd apps/srt-gui && npm run check`
- Audit i18n: `cd apps/srt-gui && npm run i18n:audit`
- Release: prefissa il commit con `feat:`/`fix:`/`perf:` e pusha su `main` — parte da solo; per un bump manuale/forzato usa Actions → "Release (semver bump)" → Run workflow.
- Pubblicazione AUR: automatica (`aur-publish.yml`) dopo che "Build and Release" completa — nessun comando manuale.
