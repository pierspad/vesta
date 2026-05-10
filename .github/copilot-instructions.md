---
description: Linee guida generali per VESTA, stack Svelte+Tauri e documentazione automatica.
---

# Istruzioni Copilot per VESTA

Queste istruzioni vengono lette automaticamente da GitHub Copilot (e altri LLM compatibili) in questo workspace grazie alla loro presenza in `.github/copilot-instructions.md`. Devi sempre aderire a queste regole quando scrivi codice o effettui delle modifiche.

## 1. Changelog Costante e Obbligatorio
Ogni volta che completi con successo un task o un bugfix significativo per l'utente, **devi categoricamente documentare le tue modifiche accodandole nel file `docs/list_of_things_changed.md`**.
* Come formattarlo: Aggiungi un punto elenco in fondo, o raggruppalo per categoria se pertinente, descrivendo *cosa* è stato cambiato.
* Perché: Questo serve all'utente, poco prima di generare una release, per compilare le note di rilascio. NON omettere questo step. Non chiedere prima il permesso all'utente, fallo automaticamente alla fine del tuo lavoro.

## 2. Traduzioni e Multi-lingua (i18n)
VESTA è supportato in 13 lingue differenti. Se metti mani alla UI (file `.svelte` o affini):
* **Non utilizzare NESSUNA stringa visibile all'utente in modo hardcoded**. Utilizza sempre la funzione `t("chiave.di.traduzione")`.
* Una volta creata in crea un piccolo script python (o usa quelli già a tua disposizione in /vesta/apps/srt-gui/scripts) per generare JSON updates tradotti in **tutte** le altre lingue presenti nella cartella `locales/` contemporaneamente (`it`, `fr`, `es`, `de`, `ar`, `ja`, `ko`, `zh`, `hi`, `ru`, `tr`, `pt`, `pl`).
* Una volta modificate le traduzioni, per validarle **DEVI eseguire sempre** il comando dal terminale:
  ```bash
  python3 apps/srt-gui/scripts/check_missing_translations.py
  ```
  Se il report terminale segnala errori, sistemali subito. Ricorda che se una traduzione in un'altra lingua (es. Francese o Italiano) coincide letteralmente con la stringa in lingua Inglese, lo script lo marcherà come `same_as_english` loggandolo come errore. In questi rari casi, usa un artefatto provvisorio (es: un accento) o segnalalo per bypassare lo script.

## 3. Comandi utili e Architettura
Vesta ha due parti principali nel workspace:
* **Frontend**: Svelte (Vite), risiede in `apps/srt-gui`. Usa `npm run dev` per i test.
* **Backend CLI/Tauri**: Rust, in `core/`, `lib/`, `cli/`, e `src-tauri/`.
* Le funzionalità core di analisi SRT sono in Rust. Non duplicare la medesima logica sul front-end se si tratta di computazione pesante, usa Tauri commands (`invoke`) chiamando Rust.
* Rust crates e dipendenze vanno lette nei rispettivi `Cargo.toml`.

## 4. Stile per SvelteKit
* Stiamo usando le **Runes di Svelte 5** (`$state`, `$derived`, `onclick` anziché `on:click`, ecc.). Non usare la sintassi obsoleta di Svelte 4, attieniti al reattivo `$state`.
* Mantieni isolato il CSS dentro il tag `<style>` e/o usa classi **Tailwind CSS**. Evita di mischiare stili globali se non strettamente necessario.

