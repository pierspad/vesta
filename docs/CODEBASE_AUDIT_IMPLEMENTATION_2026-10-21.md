# Interventi sull’audit del 21 ottobre 2026

## Baseline e release

Prima degli interventi, il checkout preesistente è stato salvato su `dev` in sette commit tematici, da `4dcbfe8` a `bad75f0`. Il push ha avviato CI e semantic-release. Sono state corrette le stringhe non tradotte rilevate dal gate i18n e applicata la formattazione Rust, senza bypass degli hook.

## Interventi

| Rilievo | Risultato |
| --- | --- |
| A01 | Cache di una coppia input/output, massimo stimato 32 MiB. Metadati size/mtime e identità inode/device su Unix invalidano la cache; cancellazione dei file e errori non servono dati vecchi. Input riusato quando cambia soltanto l’output. Ordinamento e conteggio conservati. |
| A03 | Lettura/parsing/normalizzazione, preparazione contesto e pool, anteprima su `spawn_blocking`. La guard dell’operazione resta acquisita durante la preparazione; un token cancellato impedisce le richieste provider successive. |
| A04 | Asset canonici HTML/CSS in `lib/srt-flashcards/assets/anki`, importati da Rust e GUI. Script tag/dizionario e personalizzazioni restano nei percorsi esistenti. Allowlist Vite limitata all’app e alla directory asset condivisa. |
| A05 | Unica costante del default Google: `gemini-2.5-flash`, già usato dal pool. Cambia il default del costruttore diretto; modelli scelti esplicitamente e fixture storiche restano invariati. |
| A06 | Controller condiviso per IPC, URL, riproduzione, invalidazione e cleanup. I chiamanti conservano la scelta del segmento e degli override. Test di chiusura, cambio richiesta, errori e completamenti/rejection tardivi. |
| A07 | Lettura chunk con `tokio::fs::read` in `select!` cancellabile; stesso ordine e offset, nessun aumento della concorrenza. Test con file reale, file assente e cancellazione. |
| A13 | Helper condiviso del focus editabile per le shortcut; comportamento conservato. Gli altri wrapper/markup restano invariati. |
| A16 | Costruttori fallibili aggiuntivi e uso nei percorsi già fallibili. Le firme pubbliche infallibili restano disponibili, con avviso esplicito sul fallback legacy. |

Quando un checkpoint modifica l’output, la successiva anteprima riparsa ancora l’intero file tradotto; riusa soltanto l’input. I tempi degli hit sotto misurano file invariati.

La cache usa fingerprint di metadati, non hash del contenuto: modifiche esterne che conservino deliberatamente dimensione, mtime e identità non sono rilevabili. La cancellazione delle letture async interrompe l’attesa; non promette di interrompere un’operazione filesystem già avviata dal runtime. La cancellazione durante preparazione Rust attende il worker, preservando l’esclusione dell’operazione.

## Misura riproducibile dell’anteprima

`cargo test --locked -p vesta --offline commands::translate::context_tests::preview_cache_debug_timings_for_1k_10k_and_50k_subtitles --lib -- --nocapture`

| Sottotitoli | Prima anteprima | Dieci hit |
| --- | ---: | ---: |
| 1.000 | 3,38 ms | 38,04 µs |
| 10.000 | 26,63 ms | 38,72 µs |
| 50.000 | 142,16 ms | 41,15 µs |

Misura diagnostica locale, build debug, file sintetici e cache del filesystem calda; non è un benchmark end-to-end o una stima di accelerazione della traduzione. Il test verifica dati identici/ordinati e due soli parsing totali dopo dieci hit; non impone soglie temporali.

## Rilievi lasciati aperti

- A02: durabilità per batch e ordine dei checkpoint conservati. Writer/journal richiedono misure di byte, attesa lock e throughput, oltre a test crash/failure.
- A08: serializzazione cloud conservata. Servono misure reali autorizzate e una politica di quote/concorrenza prima di modificarla.
- A09: wrapper/retry provider restano distinti; nessuna modifica al comportamento HTTP e al trimming.
- A10: deprecazioni desktop ancora presenti; migrazione e verifica nel webview/file manager reale da fare separatamente.
- A11: estratta la responsabilità audio; la ristrutturazione generale delle tab resta aperta.
- A12: CI conservata; mancano misure cold/warm per decidere condivisione artefatti o setup.
- A14: lookup ID lineare conservato, in assenza di un uso frequente misurato.
- A15: fallback legacy degli errori conservato per compatibilità.

## Validazione

Controlli prima della release: frontend check, 383 test, build, test CSS, versioni interne, formattazione, Clippy e intera suite Rust workspace (366 test passati). Hook pre-commit e pre-push eseguiti normalmente.

Controlli dopo gli interventi: suite frontend completa, Svelte/TypeScript, build, test CSS, formattazione, Clippy e suite Rust workspace; regressioni mirate della cache e del controller audio. Risultati conclusivi: 390 test frontend (76 file), zero errori/avvisi Svelte, build e test CSS riusciti, Clippy workspace senza avvisi e 380 test Rust passati (72 suite inclusi doc-test). `npm ci` ha ripristinato le dipendenze dal lockfile prima dell’ultima suite frontend. Il suo audit automatico segnala 7 vulnerabilità nelle dipendenze (2 moderate, 5 high), non analizzate o risolte in questa manutenzione. La revisione incrociata ha verificato parametri audio, invalidazione e cancellazione; il controllo primario ha ripristinato una visibilità pubblica Rust modificata involontariamente prima dei commit.

Non eseguiti test con provider a pagamento, GPU reale, Anki installato, matrice desktop o simulazioni power-loss. Modifiche concorrenti ai cataloghi e a SupportLogsPanel, comparse dopo la baseline, restano separate dai commit dell’audit.
