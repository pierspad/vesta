# Revisione Rust — 19 ottobre 2026

## Esito e metodo

Il workspace ha una buona separazione per dominio, ma alcune pipeline concentrano troppe responsabilità e alcuni invarianti non sono rappresentati né verificati. Le priorità sono correttezza di parsing/matching, protezione degli output e stato delle operazioni; le modernizzazioni sintattiche hanno un impatto minore.

Revisione del codice Rust applicativo in core/, lib/, cli/ e backend Tauri, con letture mirate del frontend per verificare i chiamanti. Due subagent Luna hanno contribuito; le loro segnalazioni sono state filtrate e controllate. Non è una verifica esaustiva del frontend, di whisper.cpp o del codice vendor, né un benchmark. Nessun sorgente applicativo modificato.

Ambiente: rustc/cargo 1.99.1, edition 2024, MSRV dichiarata 1.99. Verifiche effettuate:

- `cargo clippy --workspace --all-targets --offline -- -D warnings`: passato.
- `cargo test --workspace --offline --no-fail-fast`: passato; 324 test, 0 falliti, 0 ignorati nelle 48 suite riportate, comprese quelle senza test.
- Harness temporaneo esterno al repository, compilato con rustc contro le librerie locali e includendo i moduli parser/matcher originali: riprodotti i casi sotto.

I test passati non coprono automaticamente gli scenari nuovi identificati. Le conclusioni di concorrenza e budget derivano dalla lettura dei flussi, non da prove end-to-end con provider reali.

## Problemi da correggere

### 1. P1 — La traduzione GUI può sovrascrivere il sorgente

Riferimenti: [perform_translation](/home/ribben/Desktop/pierspad/github/vesta/apps/srt-gui/src-tauri/src/commands/translate.rs:185), [controllo già presente nella CLI](/home/ribben/Desktop/pierspad/github/vesta/cli/srt-translate-cli/src/main.rs:258).

Il comando GUI carica input e passa output alla traduzione senza `ensure_distinct_files`. Se l'utente sceglie lo stesso file, o un alias symlink, il resume lo interpreta come output esistente. Con overlap non nullo, le traduzioni successive vengono pubblicate sopra il sorgente, anche producendo un file parzialmente tradotto. La scrittura atomica evita file troncati, ma non protegge dalla scelta del file sbagliato.

Riutilizzare il controllo di identità già implementato in srt-parser prima del resume. Testare percorso identico, symlink e hard link. La funzione di traduzione che riceve solo una mappa non conosce l'identità del file di input: il controllo deve vivere nel servizio che riceve entrambi i percorsi o negli adapter.

### 2. P1 — Ricerca binaria su una proprietà non ordinata

Riferimenti: [match_subtitles](/home/ribben/Desktop/pierspad/github/vesta/lib/srt-flashcards/src/matcher.rs:16), [ordinamento del parser](/home/ribben/Desktop/pierspad/github/vesta/lib/srt-flashcards/src/parser.rs:48).

Il parser ordina per `start_ms`, ma `partition_point` cerca su `end_ms`. Con intervalli annidati, il predicato non è monotono: può saltare il miglior candidato.

Riproduzione: sottotitolo sorgente [10000,11000]; candidati ordinati per inizio [0,20000], [1000,2000], [3000,4000]. Il risultato è `None`, benché il primo candidato copra completamente il sorgente. È un caso valido per sottotitoli sovrapposti, non solo per file corrotti.

Correzione: scansione corretta come riferimento, quindi indice di intervalli o massimi prefissi delle fini con ricerca monotona. Ordinare soltanto per fine non basta, perché il successivo `break` dipende dall'ordinamento per inizio. Aggiungere test con intervalli annidati e confronto con una versione semplice di riferimento.

### 3. P2 — Normalizzazione con ID zero: underflow

Riferimento: [normalize_subtitles](/home/ribben/Desktop/pierspad/github/vesta/core/srt-parser/src/lib.rs:254).

Il parser accetta `0` come ID. Con il solo ID 0, o con ID consecutivi da 0, `max_id as u64 - subtitles.len() as u64` va in underflow. Riprodotto il panic in debug; con i normali overflow check disabilitati in release, il valore avvolge e fa saltare la normalizzazione.

Decidere una politica esplicita: rifiutare ID zero oppure supportare numerazioni da zero. Calcolare poi il numero di ID mancanti sul dominio corretto. `saturating_sub` evita il panic ma, da solo, non definisce la semantica. Testare anche ID duplicati: oggi l'inserimento nella HashMap sostituisce silenziosamente la voce precedente.

### 4. P2 — Il parser ASS inventa timestamp validi

Riferimento: [parse_ass_timestamp](/home/ribben/Desktop/pierspad/github/vesta/lib/srt-flashcards/src/parser.rs:209).

Ogni componente mancante o invalido usa `unwrap_or(0)` e la funzione restituisce sempre `Ok`. Riproduzione: `parse_ass_timestamp("garbage") == Ok(0)`. Anche il chiamante usa `unwrap_or(0)`, nascondendo eventuali errori futuri. Ne risultano cue al tempo zero o con durate invalide. L'aritmetica su componenti i64 non validati può inoltre andare in overflow con input estremi.

Validare numero e formato dei componenti, intervalli e aritmetica; propagare errori con posizione della riga, oppure adottare una modalità tollerante che scarti la riga con diagnostica. Non trasformare errori in tempi reali.

Nel parser SRT c'è un problema affine di validazione: `00:75:00,000` viene accettato e riscritto come `01:15:00,000`. Se la tolleranza è intenzionale va documentata e distinta dalla modalità rigorosa. Non c'è invece l'ipotizzato overflow in `Timestamp::new`: i componenti u32 sono convertiti a u64 prima dell'aritmetica e il massimo risultato entra nel tipo; questa segnalazione è stata scartata.

### 5. P2 — Anteprime concorrenti scrivono nello stesso file

Riferimenti: [audio preview](/home/ribben/Desktop/pierspad/github/vesta/apps/srt-gui/src-tauri/src/commands/flashcards/commands.rs:93), [snapshot preview](/home/ribben/Desktop/pierspad/github/vesta/apps/srt-gui/src-tauri/src/commands/flashcards/commands.rs:129).

I nomi temporanei contengono solo il PID, e per l'audio l'estensione. Richieste sovrapposte nello stesso processo condividono la destinazione FFmpeg. Il frontend controlla che una risposta appartenga alla richiesta corrente, ma una richiesta vecchia può comunque riscrivere il file che la risposta nuova sta servendo. Possibili anteprime sbagliate o letture durante la scrittura.

Usare un percorso univoco per richiesta e gestire esplicitamente la sua durata: il file deve restare disponibile dopo il ritorno del comando fino alla lettura del player. Un TempDir locale distrutto al ritorno non risolverebbe correttamente il problema. Test di regressione: due richieste con completamento invertito.

### 6. P2 — Auto-sync può applicare risultati a un documento diverso

Riferimenti: [snapshot e applicazione auto-sync](/home/ribben/Desktop/pierspad/github/vesta/apps/srt-gui/src-tauri/src/commands/auto_sync.rs:58), [sostituzione engine](/home/ribben/Desktop/pierspad/github/vesta/apps/srt-gui/src-tauri/src/commands/sync.rs:45).

Auto-sync copia sottotitoli/media, rilascia il mutex durante il lavoro e alla fine applica le proposte all'engine presente in quel momento. `sync_load_srt` e `sync_set_video` non verificano l'operazione attiva e possono cambiare il contesto. Se il nuovo file riusa gli stessi ID, le ancore possono essere applicate al documento sbagliato. Il mutex evita una data race, non l'incoerenza logica tra snapshot e commit.

Introdurre una revisione/sessione del documento verificata prima del commit, oppure impedire le mutazioni incompatibili durante l'operazione. Testare il caricamento del documento B mentre è in corso l'analisi di A. Verificare anche la cancellazione prima dell'applicazione finale.

### 7. P2 — Il repair rinnova il budget dei provider

Riferimenti: [scheduler iniziale](/home/ribben/Desktop/pierspad/github/vesta/lib/srt-translate/src/lib.rs:569), [nuovo scheduler nel repair](/home/ribben/Desktop/pierspad/github/vesta/lib/srt-translate/src/lib.rs:845), [contabilizzazione](/home/ribben/Desktop/pierspad/github/vesta/lib/srt-translate/src/scheduler.rs:44).

`TierScheduler::new` riparte da `max_requests` e da `exhausted=false`. Il repair crea una nuova istanza anziché riutilizzare quella della fase batch. Un endpoint con budget 1 può quindi ricevere una chiamata batch e un'altra nel repair; un endpoint marcato esaurito può essere riattivato. Questo contraddice un budget inteso per l'intera operazione, come suggerisce la configurazione, anziché per fase.

Condividere lo scheduler per tutta l'operazione. Specificare inoltre se il limite conta assegnazioni di lavoro o vere richieste HTTP, perché i retry interni del translator sono un livello distinto. Aggiungere test con server locale e contatore delle richieste; non serve un provider reale.

### Ulteriore hardening: estrazione ZIP

[unzip_to](/home/ribben/Desktop/pierspad/github/vesta/core/srt-apkg/src/lib.rs:54) filtra i percorsi delle entry con `enclosed_name`, ma segue eventuali symlink già presenti nella destinazione quando crea i file. Il rischio richiede una destinazione preesistente manipolabile; le directory temporanee nuove usate internamente lo riducono. Rendere esplicito il contratto del metodo pubblico e preferire destinazioni nuove/private o estrazione resistente ai symlink. Non è stata dimostrata una compromissione del flusso GUI ordinario.

## Manutenibilità, duplicazione e SRP

### Componenti già ben separati

- `OperationGuard` e `OperationState` centralizzano acquisizione, cancellazione e rilascio delle operazioni, con test del lifecycle. La macro per gli stati Tauri conserva le identità di tipo richieste dal registry.
- `srt-parser::persistence`, `srt-download` e `srt-apkg` centralizzano infrastruttura realmente condivisa; la pubblicazione atomica ha un contratto esplicito.
- Translation pool, scheduler, rate limiter e prompt hanno moduli distinti. Le API legacy di traduzione delegano già al percorso nuovo in punti importanti: non sono tutte implementazioni da riscrivere.
- Le librerie espongono callback e token senza dipendere da Tauri; GUI e CLI possono riusare i motori.

### Refactoring ad alto rendimento

1. **Flashcard generation** — [generate](/home/ribben/Desktop/pierspad/github/vesta/lib/srt-flashcards/src/lib.rs:322) arriva quasi alla fine del file: pianificazione, scelta encoder, staging, scheduling dei media, avanzamento, verifica errori ed export sono nello stesso flusso. Separare `GenerationPlan`, esecuzione media con esito strutturato e pubblicazione. Il coordinatore dovrebbe descrivere queste fasi. Conservare ownership di staging e regole di cancellazione: estrarre solo blocchi in helper con molti argomenti trasferirebbe la complessità.

2. **APKG** — [generate_apkg](/home/ribben/Desktop/pierspad/github/vesta/lib/srt-flashcards/src/export_apkg.rs:50) mescola schema SQLite, modelli Anki, template, campi note, nomi media e ZIP. Separare modello/template, costruzione dei record, scrittura DB e packaging. La stessa funzione costruisce una grande stringa SQL con escaping manuale: transaction e statement parametrizzati riducono memoria intermedia, lavoro di parsing e superficie di manutenzione. Misurare il beneficio prima di attribuire un guadagno prestazionale.

3. **Parsing SRT duplicato** — [core parser](/home/ribben/Desktop/pierspad/github/vesta/core/srt-parser/src/lib.rs:156) e [flashcard parser](/home/ribben/Desktop/pierspad/github/vesta/lib/srt-flashcards/src/parser.rs:23) duplicano separazione blocchi, ID, timeline e testo. Hanno però politiche differenti: errore o skip, placeholder o esclusione del testo vuoto, HashMap o Vec. Estrarre un parser comune che produca cue ordinabili più diagnostica; mantenere esplicite le politiche dei consumatori. Non sostituire semplicemente un parser con l'altro cambiando il comportamento.

4. **Errori e configurazioni** — il codice riconosce cancellazione con sottostringhe in [translate command](/home/ribben/Desktop/pierspad/github/vesta/apps/srt-gui/src-tauri/src/commands/translate.rs:268), e rate limit con ricerca testuale nello scheduler. Introdurre errori tipizzati nel dominio e convertire in stringhe soltanto al confine CLI/Tauri. Per provider, formato export e backend usare enum interni validati, traducendo gli attuali DTO tramite `TryFrom`. Per esempio `TranscriptionConfig` combina opzioni locali/cloud che non sono tutte compatibili.

5. **Adapter GUI** — il suggerimento del contesto traduttivo contiene campionamento, prompt e failover nel comando Tauri. Spostare il servizio nella libreria, lasciando al comando gli eventi e la conversione degli errori. Risolvere gli strumenti media in un modulo applicativo dedicato eviterebbe che trascrizione e auto-sync dipendano dal namespace dei comandi flashcard.

La lunghezza del file è un indicatore, non una violazione automatica di SRP: alcuni file includono centinaia di righe di test. Non serve creare un crate per ogni piccolo helper né introdurre trait per un'unica implementazione senza un confine utile.

## Rust moderno: 1.95–1.99

Il codice è già idiomatico in più punti: let-else, let chains, slice patterns, `is_some_and`/`is_none_or`/`is_ok_and`, `div_ceil`, `LazyLock`, RAII, `JoinSet`, cancellation token, `Cow` e `mem::take`. Questi strumenti non sono tutti novità di 1.95–1.99, ma il loro uso conta più del numero di API recenti adottate.

| Versione | Opportunità concreta e valutazione |
| --- | --- |
| 1.95 | `cfg_select!` può rendere mutuamente esclusivi i rami di scelta encoder in `media.rs:197`; if-let guard nei match solo dove eliminano annidamenti reali. [Release ufficiale](https://blog.rust-lang.org/2026/04/16/Rust-1.95.0/). |
| 1.96 | `assert_matches!` rende più informativi test oggi scritti come `assert!(matches!(...))`, ad esempio in `srt-refine/src/cards.rs:790`. I nuovi tipi `core::range` sono utili se un'API deve memorizzare intervalli Copy; nessun motivo per convertire tutti i loop. [Release ufficiale](https://blog.rust-lang.org/2026/05/28/Rust-1.96.0/). |
| 1.97 | Cargo può gestire i warning con `CARGO_BUILD_WARNINGS=deny` senza invalidare la cache come una variazione di rustflags. Valutazione per la CI, mantenendo la copertura Clippy; scarso impatto sulla leggibilità del sorgente. [Release ufficiale](https://blog.rust-lang.org/2026/07/09/Rust-1.97.0/). |
| 1.98 | `NumBuffer`/`format_into` solo se il profiling individua nella formattazione interi un costo significativo. I metodi floating-point algebraici cambiano le libertà di ottimizzazione numerica: non li introdurrei automaticamente nel matching temporale. [Release ufficiale](https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/). |
| 1.99 | `String::from_utf8_lossy_owned` è **già usato** in `srt-extract/src/embedded.rs:176`. È applicabile anche a stderr posseduto dove oggi si fa `from_utf8_lossy(...).into_owned()`, ad esempio `commands/flashcards/commands.rs:470`. Le nuove API unsafe/FFI non danno un vantaggio evidente a questi flussi. [Release ufficiale](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/). |

Il workspace richiede attualmente Rust 1.99: non dichiara compatibilità con 1.95. L'uso di `from_utf8_lossy_owned` è una ragione concreta per quel requisito. Una compatibilità con tutte le versioni 1.95–1.99 richiederebbe un lavoro distinto sulla MSRV e le dipendenze.

Piccoli miglioramenti indipendenti dalla versione: `map_subtitle_to_info` restituisce sempre `Some`, quindi può restituire direttamente `SubtitleInfo` e usare `map`; il calcolo manuale dell'indice iniziale in `get_latest_translated_subtitles` può usare `saturating_sub`; tipi `NonZeroUsize` per batch validati e nuovi tipi per millisecondi/ID renderebbero gli invarianti più espliciti. Non mescolare questi ritocchi con le correzioni funzionali.

## Prestazioni e reattività

- La traduzione salva l'intera mappa a ogni batch mentre detiene il mutex dei risultati (`lib/srt-translate/src/lib.rs:682`). Il salvataggio comprende serializzazione, scrittura sincrona e sync del file. Al crescere dei batch aumenta il lavoro cumulativo e il lock serializza i worker. Valutare un unico writer con snapshot versionati e checkpoint per numero di batch/tempo, mantenendo un checkpoint finale e il comportamento di resume. Non lanciare writer indipendenti che possano pubblicare uno snapshot vecchio dopo uno nuovo.
- L'export APKG viene chiamato sincronicamente dall'async `generate` (`lib/srt-flashcards/src/lib.rs:935`): SQLite, copia media e ZIP possono occupare a lungo un worker Tokio. Spostare l'export in `spawn_blocking` con ownership dello staging. Il solo offload non rende cancellabile la fase: occorrono controllo del token prima della pubblicazione e gestione del task fino alla sua conclusione.
- `get_latest_translated_subtitles` rilegge e riparsa entrambi i file per costruire poche righe di preview. Prima di introdurre cache, misurare dimensione dei file e frequenza delle chiamate; una cache deve invalidarsi al cambio documento/output.
- Ridurre copie complete nella traduzione (`sorted`, `subtitles_map`, batch posseduti, `existing.clone()`) dopo aver chiarito l'ownership. Sono ottimizzazioni di memoria possibili, non colli di bottiglia misurati: normalmente rete e FFmpeg dominano.

## Sequenza proposta

1. Correggere protezione input/output, matcher e parsing con test di regressione minimi.
2. Correggere percorsi preview, revisione documento auto-sync e budget condiviso con test deterministici.
3. Estrarre pianificazione/esecuzione/pubblicazione flashcard e parametrizzare SQLite mantenendo gli output golden.
4. Unificare parsing, errori e configurazioni interne preservando i contratti pubblici.
5. Applicare modernizzazioni locali e misurare checkpoint/export prima e dopo.

Non raccomando una riscrittura completa: i confini dei crate, la pubblicazione atomica e la copertura dei lifecycle esistenti sono una base utile da conservare.
