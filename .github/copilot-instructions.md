---
description: Linee guida LLM per VESTA, i18n, qualita e release.
---

# Istruzioni Copilot per VESTA

Queste istruzioni vengono lette automaticamente da GitHub Copilot e da altri LLM compatibili quando lavorano in questo workspace. Devi aderire a queste regole quando scrivi codice, aggiorni documentazione o prepari una release.

## 1. Conventional Commits obbligatori

I messaggi di commit SONO le release notes: `@semantic-release/release-notes-generator` genera il body della GitHub Release e il `CHANGELOG.md` direttamente dai commit. Non esistono file di note manuali da aggiornare.

- Usa sempre il formato Conventional Commits: `tipo(scope): descrizione`.
  - `feat:` → minor bump, sezione "✨ New Features";
  - `fix:` → patch bump, sezione "🐛 Bug Fixes";
  - `perf:` → patch bump, sezione "🔧 Improvements";
  - `bump:` → patch bump, sezione "🚀 Updated App Support";
  - `feat!:`/`fix!:` o footer `BREAKING CHANGE:` → major bump;
  - `chore:`/`docs:`/`refactor:`/`test:`/`ci:` → nessun rilascio.
- Scrivi la descrizione del commit pensando all'utente finale che la leggerà nella release: sintetica, concreta, in inglese, con lo scope che indica l'area toccata (es. `fix(shortcuts): restore Ctrl+S on Windows`).
- Il dettaglio tecnico/operativo va nel body del commit, non serve altrove.

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

## 5. Release e pubblicazione

Il rilascio è interamente gestito da **semantic-release** (`release.yml`, config in `.releaserc`). Nessuno script locale, nessun file di note manuale.

- **Push su `main`** → release stabile `vX.Y.Z`, marcata Latest su GitHub e pubblicata su AUR.
- **Push su `dev`** → prerelease `vX.Y.Z-dev.N`, marcata Pre-release su GitHub: non diventa mai Latest e non va mai su AUR. Il contatore `.N` cresce a ogni push su dev; la versione base si consolida al merge su main.
- Il body della release e `CHANGELOG.md` sono generati dai Conventional Commits (§1). Non riscriverli a mano.
- Il PKGBUILD (`build-scripts/PKGBUILD`) è la Single Source of Truth della versione: `update_project_info.sh` la propaga a tauri.conf.json, Cargo.toml, package.json, .desktop e flatpak. Non bumpare versioni a mano.
- Flusso: semantic-release calcola il bump → propaga versione → commit `chore: Release vX.Y.Z [skip ci]` + tag → crea la GitHub Release → dispatcha "Build and Release" sul tag (upload binari) → backmerge `main → dev`.

## 6. Comandi utili

- Frontend: `cd apps/srt-gui && npm run check`
- Audit i18n: `cd apps/srt-gui && npm run i18n:audit`
- Release: prefissa il commit con `feat:`/`fix:`/`perf:`/`bump:` e pusha — parte da sola (stabile da `main`, prerelease da `dev`).
- Pubblicazione AUR: automatica (`aur-publish.yml`) dopo che "Build and Release" completa su un tag stabile — nessun comando manuale.
