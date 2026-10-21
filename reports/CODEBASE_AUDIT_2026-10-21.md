# Audit del codebase Vesta

Data: 21 ottobre 2026  
Commit analizzato: `e8718b71683c884fca7645e3e76b94d542b93176`  
Versione applicazione: `0.29.0-dev.1`  
Stato: analisi per revisione; nessuna modifica applicata al codice o alle dipendenze.

## 1. Valutazione complessiva

Il repository ha già una separazione utile fra core, motori, CLI e desktop, helper condivisi e una consistente copertura di regressione. I maggiori margini di miglioramento emergono nel frontend: discovery dei modelli implementata con politiche differenti, configurazioni replicate, componenti molto estesi e alcune responsabilità residue non utilizzate.

Le ottimizzazioni Rust più interessanti riguardano checkpoint di traduzione e I/O degli archivi. Sono costi identificabili nel codice, ma questo audit non dimostra che dominino il tempo reale di elaborazione. La rimozione indiscriminata di API legacy, benchmark, fallback o copie transitive delle dipendenze sarebbe ingiustificata.

Priorità iniziale proposta: verificare la discovery dei tier, correggere l'identità delle cache e isolare i piccoli residui inutilizzati. Successivamente valutare deduplicazioni e misurare i percorsi costosi. Nessuna delle proposte costituisce una decisione già approvata.

## 2. Perimetro, metodo e limiti

Inventario dei sorgenti tracciati sotto `core/`, `lib/`, `cli/`, `apps/`, `build-scripts/`, esclusi i percorsi `gen`:

| Formato | File | Righe fisiche |
|---|---:|---:|
| Rust | 128 | 31.792 |
| TypeScript | 194 | 16.039 |
| Svelte | 93 | 23.609 |
| Python | 12 | 1.918 |
| Totale | 427 | 73.358 |

Il workspace contiene 22 package Cargo. I conteggi comprendono test e commenti; non sono una misura della complessità. Non comprendono JavaScript, JSON, shell, YAML, codice vendorizzato e asset, alcuni dei quali sono stati comunque esaminati nei percorsi pertinenti.

Metodo: inventario e dimensionamento; ricerca di blocchi identici di 12 righe normalizzate; ricerca di simboli legacy e soppressioni; lettura mirata di motori, adapter, workflow e componenti; confronto con ARCHITECTURE, MAINTENANCE, QUALITY e CI; esecuzione dei controlli elencati sotto. I duplicati testuali sono stati filtrati manualmente per distinguere logica, markup, fixture e icone.

È una revisione trasversale con approfondimenti mirati, non una lettura integrale di ogni riga né un'analisi formale del grafo dei riferimenti. Non è stato eseguito un rilevatore AST completo di codice morto o cloni. Assenza di chiamanti interni non significa assenza di consumatori esterni. La percentuale complessiva di duplicazione non è stata misurata.

Non sono stati eseguiti: intero workspace nativo, Clippy dell'intero workspace, applicazione installata, provider autenticati, inferenza GPU, benchmark end-to-end, audit online aggiornato delle dipendenze. Nessuna affermazione sull'obsolescenza attuale di modelli o librerie deriva dal loro nome o numero di versione. La directory vendor è esclusa dalle proposte di pulizia automatica.

### Verifiche effettivamente eseguite

| Controllo | Esito |
|---|---|
| `npm run check` | 0 errori, 0 warning |
| `npm test` | 94 file, 454 test superati |
| `npm run build` | riuscita; 355 moduli trasformati |
| `cargo test --locked -p srt-parser -p srt-sync -p srt-translate -p srt-refine` | 113 test superati; doctest senza casi |
| `cargo clippy --locked -p srt-parser -p srt-sync -p srt-translate -p srt-refine --all-targets -- -D warnings` | superato |
| `cargo fmt --all --check` | superato |
| `python3 build-scripts/check_tauri_versions.py` | coppie bloccate compatibili |
| `python3 build-scripts/test_tauri_versions.py` | 5 test superati |
| `bash build-scripts/check_internal_crate_versions.sh` | versioni coerenti |
| `cargo tree --locked --offline --duplicates` | consultato per dipendenze transitive duplicate |

I test frontend usano ambiente simulato; non certificano comportamento del webview nativo. I comandi hanno prodotto artefatti locali di build; nessun sorgente applicativo è stato cambiato.

## 3. Indice dei rilievi

P1 = verificare prima del refactoring estetico; P2 = miglioramento funzionale/manutentivo da pianificare; P3 = pulizia o ottimizzazione subordinata a misure. «Confermato» descrive l'evidenza nel sorgente, non una riproduzione completa nell'app installata.

| ID | Priorità | Area | Rilievo | Evidenza |
|---|---|---|---|---|
| A01 | P1 | Tier LLM | possibile ciclo reattivo di discovery locale | ipotesi forte da riprodurre |
| A02 | P1 | Tier LLM | cache non invalidata ruotando credenziali | confermata nel codice |
| A03 | P2 | Discovery gestita | chiave cache parziale e nessuna scadenza | confermata nel codice |
| A04 | P2 | Tier STT | richieste concorrenti senza protezione dall'ordine inverso | confermata nel codice |
| A05 | P2 | Configurazione | parser dei tier duplicati | confermata |
| A06 | P3 | STT | handler numerico senza chiamanti e campi inattivi | confermata nel percorso esaminato |
| A07 | P2 | Disponibilità LLM | probe ripetuti dello stesso endpoint | confermata nel codice |
| A08 | P2 | Refine | cache testuale senza limite o reset | confermata nel codice |
| A09 | P3 | Refine | normalizzazione ripetuta a ogni ricerca | confermata; costo da misurare |
| A10 | P2 | Traduzione | riscrittura integrale dei checkpoint sotto lock | intenzionale; costo da misurare |
| A11 | P3 | Matching | caso peggiore quadratico con cue lunghi | dedotto dall'algoritmo |
| A12 | P2 | Frontend | tab ancora molto estesi | confermata |
| A13 | P3 | Interfaccia | markup e flussi di presentazione ripetuti | confermata |
| A14 | P2 | Cataloghi | metadati Whisper duplicati e divergenti | confermata |
| A15 | P3 | Archivi | media numerati ricompressi nel percorso Refine | confermata per file numerati senza estensione |
| A16 | P3 | Merge APKG | riletture e scritture temporanee evitabili | confermata; costo da misurare |
| A17 | P3 | CI | setup e build frontend ripetuti tra job | confermata; tradeoff da valutare |
| A18 | P3 | Test/tooling | fixture di cancellazione e helper locale duplicati | confermata |

## 4. Rilievi dettagliati

### A01 — Possibile circuito reattivo nella discovery locale

**Evidenza:** `apps/srt-gui/src/lib/components/TranslationTiers.svelte:58`, `:113`, `:127`. L'effetto chiama `ensureModels`; prima del primo `await`, la funzione legge `discovered.has(cacheId)`. Dopo una discovery riuscita assegna una nuova Map a `discovered`. Per il provider `local` il controllo di cache non interrompe la funzione. `inflight` deduplica solo le richieste contemporaneamente pendenti; viene liberato alla fine.

**Impatto possibile:** l'aggiornamento di una dipendenza letta dall'effetto può riattivare la discovery a ogni risposta, superando la cadenza di 15 secondi prevista dal timer. È un rischio specifico del collegamento reattivo, non un rallentamento misurato durante questo audit.

**Proposta da valutare:** separare gli input reattivi dalla cache di output; un solo scheduler esplicito per il polling locale e una richiesta pendente per identità. Evitare una modifica indiscriminata agli effetti degli altri workflow.

**Validazione preliminare:** montare il componente reale con un endpoint locale simulato, restituire subito una lista valida e contare le richieste prima dei 15 secondi. Testare anche risposta vuota, errore e smontaggio. Priorità alta di verifica; non classificato come bug riprodotto nell'app.

### A02 — La cache dei tier LLM non segue le credenziali

**Evidenza:** `TranslationTiers.svelte:53–105`. L'identità è `provider::apiKeyId::apiUrl`; non comprende una revisione della credenziale. Il componente ricarica le chiavi su `apikeys-updated`, ma una voce remota già presente in `discovered` termina prima della discovery. Le risposte pendenti non hanno un controllo di generazione/disposal prima di aggiornare memoria e persistenza.

**Scenario:** cambiare il segreto mantenendo lo stesso ID e URL lascia valida la lista del precedente account. Una richiesta avviata prima del cambio può inoltre pubblicare la vecchia lista nella stessa identità.

**Proposta:** identità completa a livello logico, invalidazione al cambio credenziali e filtro delle risposte obsolete; evitare di inserire segreti in chiaro nei nomi delle cache. Riutilizzare i principi di `LatestRequest` o `ManagedModelDiscovery`, preservando discovery per più endpoint.

**Verifica:** cambio segreto sul medesimo ID, cambio URL, rimozione entry durante richiesta, risposte in ordine inverso e teardown. Controllare memoria e cache persistente, non solo la lista visualizzata.

### A03 — Cache persistente gestita senza TTL e con identità troncata

**Evidenza:** `apps/srt-gui/src/lib/workflows/managedModelDiscovery.svelte.ts:24–38`. L'identità in memoria usa la chiave completa, quella persistita solo i primi otto caratteri. Le liste valide non hanno timestamp o scadenza.

**Impatto:** due chiavi con uguale prefisso possono condividere dati persistiti; modelli aggiunti/rimossi possono restare nascosti fino al refresh esplicito. Il filtro delle risposte obsolete già presente è corretto e va conservato.

**Proposta:** ID o revisione della credenziale, schema cache versionato, TTL e refresh esplicito. Documentare la politica unica rispetto alle 24 ore usate dai tier.

**Verifica:** riavvio con due credenziali dallo stesso prefisso, cache scaduta e malformata, cambio provider e refresh manuale. Nessuna conclusione sulla reale disponibilità dei modelli cloud senza interrogare il provider.

### A04 — Discovery STT senza deduplicazione e controllo della risposta più recente

**Evidenza:** `apps/srt-gui/src/lib/components/TranscribeTiers.svelte:261–287`. `discoverSpeechModels` aggiorna `speechModels` e `speechDiscoveryError` dopo ogni risposta; non usa `inflight`, versione o token di validità. La chiave dipende da ID credenziale e URL, non dalla sua revisione.

**Impatto:** due refresh della stessa identità possono completarsi in ordine inverso; una risposta vecchia può sostituire quella nuova. Anche il semplice esito di errore può diventare obsoleto.

**Proposta:** gestione delle richieste per identità e rifiuto dei risultati superati, con invalidazione al cambio chiave. Condividere la responsabilità asincrona, non l'intero controller STT/LLM.

**Verifica:** due richieste differite con esiti invertiti, rotazione del segreto e rimozione dell'entry. I test esistenti per gli altri controller non coprono automaticamente questo componente.

### A05 — Duplicazione dei parser e della persistenza dei tier

**Evidenza:** `apps/srt-gui/src/lib/config/translationTiers.ts:44–86` e `transcribeTiers.ts:26–77`: validazione array, ID mancanti, entry, provider, modello, RPM e budget ripetuti, con uso di `any[]`.

**Impatto:** correzioni e regole di normalizzazione possono divergere. La duplicazione è semantica anche quando le righe non coincidono per formattazione.

**Proposta:** un parser puro condiviso con parametri per tipi/provider e default, adapter separati per chiave persistita ed evento. Conservare il default STT locale `base` e il default LLM vuoto: sono differenze intenzionali.

**Verifica:** fixture comuni per input corrotti e tipi errati; test separati di default, eventi, compatibilità e salvataggio. Non estendere automaticamente le regole di esecuzione LLM alla trascrizione.

### A06 — Residui di configurazione numerica STT

**Evidenza:** `TranscribeTiers.svelte:232–254` contiene `onNumberChange` senza riferimenti nel componente. `onDestroy` è importato ma non utilizzato. `TranscribeTierEntry` persiste `rpm` e `maxRequests`, ma `apps/srt-gui/src/lib/workflows/transcription.ts` non li applica e `buildTranscriptionRequest` non li passa al backend.

**Impatto:** rumore manutentivo e schema che suggerisce semantiche non implementate nel percorso attuale. Non è stata trovata una UI attuale che prometta tali limiti: non è corretto dichiarare un controllo utente sicuramente ingannevole.

**Proposta:** eliminare l'handler e l'import inutilizzati in un intervento piccolo; decidere separatamente se mantenere i campi per compatibilità o implementarli con una semantica esplicita. Non rimuovere dati salvati senza una decisione di compatibilità.

**Verifica:** check Svelte, test tier e workflow STT; ricerca completa dei riferimenti prima della rimozione.

### A07 — Probe di disponibilità duplicati per endpoint

**Evidenza:** `apps/srt-gui/src/lib/config/llmTiers.ts:104–146`. `Promise.all` avvia un controllo per ogni entry configurata. Più modelli o tier sulla stessa URL/credenziale ripetono la richiesta di discovery. La funzione attende anche i probe locali quando una voce remota con chiave soddisfa già il criterio corrente di disponibilità.

**Impatto:** traffico e latenza evitabili, soprattutto con endpoint locali lenti e molti modelli. Il tempo dipende da timeout e rete; non è stato misurato.

**Proposta:** risolvere una volta le identità, deduplicare i probe entro il singolo controllo e valutare l'uscita anticipata per remote già disponibili secondo la semantica esistente. Evitare cache globali che rendano obsoleto lo stato di readiness.

**Verifica:** più entry sul medesimo endpoint → un probe; credenziali diverse → controlli distinti; preservare fallback 404/405 e classificazione degli errori.

### A08 — Cache HTML di Refine che cresce per tutta la vita del tab

**Evidenza:** `apps/srt-gui/src/lib/workflows/refineCardSearch.ts:14–36`: `htmlStripCache` conserva ogni testo incontrato e non espone reset/limite. `RefineTab.svelte:135` crea una sola istanza. App mantiene montati i tab già caricati.

**Impatto:** aprendo diversi deck si conservano anche testi non più presenti. Non è una fuga di memoria fuori controllo del runtime: è ritenzione applicativa priva di una politica di espulsione.

**Proposta:** cache legata al dataset corrente, reset al cambio deck o limite LRU. Conservare il riuso utile durante modifiche e ricerca sullo stesso dataset.

**Verifica:** caricare sequenzialmente deck con testi disgiunti, misurare heap e dimensione cache; verificare ricerca dopo aggiornamento note e cambio file.

### A09 — Ricerca Refine: normalizzazione ripetuta

**Evidenza:** `refineCardSearch.ts:39–53`. A ogni filtro vengono ripetuti `toLowerCase()` per espressione, significato e note di tutte le carte. L'indice esistente conserva testi senza HTML ma non quelli normalizzati. La lista UI è già virtualizzata (`RefineTab.svelte:147` circa), quindi il rendering non va descritto come integralmente non virtualizzato.

**Proposta:** indicizzare il testo normalizzato e aggiornare le note mutate; eventuale debounce solo se il profilo interattivo lo giustifica.

**Verifica:** confrontare p50/p95 delle ricerche su 1k/10k/50k carte, con note grandi, Unicode e modifiche in corso. Valutare memoria extra e invalidazione; priorità subordinata a misure.

### A10 — Checkpoint di traduzione integrali e serializzati

**Evidenza:** `lib/srt-translate/src/lib.rs:680–688`, `:931–943`; `core/srt-parser/src/lib.rs:324–336`. Ogni batch aggiorna la mappa, ordina e serializza l'intero contenuto completato e lo pubblica mantenendo il lock posseduto dal worker bloccante.

**Costo:** con N righe e batch B, il numero cumulativo di righe riscritte cresce nell'ordine di N²/B per batch di dimensione costante. L'ordinamento si aggiunge alla serializzazione. Il lock può serializzare i completamenti, ma garantisce l'ordine dei checkpoint anche quando una future viene abbandonata.

**Proposta:** prima misurare il benchmark già presente `lib/srt-translate/examples/checkpoint_bench.rs`, byte cumulativi e attesa del lock sullo storage reale. Solo se il costo è significativo valutare un writer ordinato o journal con compattazione e contratto di durability definito.

**Vincolo:** non liberare semplicemente il lock prima della scrittura e non ridurre la frequenza senza decidere quanta progressione si può perdere. MAINTENANCE documenta esplicitamente questa scelta.

**Verifica:** ordine invertito dei completamenti, errore disco, cancellazione/drop, ripresa e preservazione dell'output; benchmark identico prima/dopo. Nessun aumento automatico di worker.

### A11 — Matching temporale: degradazione con intervalli molto lunghi

**Evidenza:** `lib/srt-flashcards/src/matcher.rs:10–47`. I massimi prefissi permettono una ricerca iniziale corretta anche con fine dei cue non monotona. Un cue iniziale lunghissimo può però mantenere basso `search_start`, facendo riesaminare molti candidati per ogni riga target.

**Impatto:** caso peggiore O(N×M); non rappresenta necessariamente SRT ordinari. Il codice contiene già test specifici per cue annidati: la correttezza non va sacrificata usando una ricerca binaria sulle sole fini.

**Proposta:** misurare input rappresentativi e un caso avverso; considerare sweep line o indice di intervalli solo se necessario.

**Verifica:** confronto con riferimento esaustivo, tie, cue annidati, timestamp estremi e confronto prestazioni su film reali.

### A12 — Tab frontend ancora concentrati in file grandi

| File | Righe fisiche |
|---|---:|
| FlashcardsTab.svelte | 1.773 |
| SyncTab.svelte | 1.473 |
| TranscribeTab.svelte | 1.356 |
| SettingsTab.svelte | 1.334 |
| TranslateTab.svelte | 1.310 |
| RefineTab.svelte | 1.259 |

**Impatto:** maggiore costo di revisione e più dipendenze fra preferenze, effetti, eventi e markup. La dimensione da sola non dimostra inefficienza runtime o cattiva architettura; i workflow estratti sono già una base positiva.

**Proposta:** estrarre responsabilità complete e verificabili: discovery dei tier, sezioni di impostazioni, presentazione dei risultati. Mantenere esplicite ownership delle operazioni, differenza fra hidden e destroy, cleanup e invalidazione.

**Verifica:** regressioni montate su persistenza stato, scorciatoie del tab inattivo, cancellazione e teardown. Suddividere le PR per responsabilità, senza riscrittura generale.

### A13 — Presentazione ripetuta nei componenti

**Evidenza:** blocchi ripetuti in `TranslationTiers.svelte` e `TranscribeTiers.svelte` (entry e aggiunta endpoint), negli stati risultato di `TranslateTab.svelte:1125` e `TranscribeTab.svelte:1162`, nel download FFmpeg di `FlashcardsTab.svelte:1480` e `TranscribeTab.svelte:693`, oltre a SVG inline ripetuti. I numeri indicano finestre del rilevamento testuale, non il confine intero del blocco.

**Proposta:** piccoli componenti di presentazione dopo confronto delle differenze; partire da stati risultato e requisito FFmpeg. `TierList`, `FooterActions`, `PathPickerField` e altri componenti condivisi esistono già: estenderli dove appropriato.

**Impatto:** uniformità e minori correzioni replicate; il guadagno sul bundle non è quantificato. Non fondere i flussi di cancellazione STT e LLM solo perché il markup somiglia.

**Verifica:** accessibilità, traduzioni, stati busy/error/success e callback nei due chiamanti.

### A14 — Catalogo Whisper locale duplicato

**Evidenza:** `TranscribeTiers.svelte:26–32` contiene una lista statica equivalente a `lib/srt-transcribe/src/model.rs:16–22`, con dimensioni diverse (`~142MB` contro `~150 MB`, `~466MB` contro `~500 MB`). La lista frontend è sostituita dal risultato IPC quando disponibile.

**Impatto:** metadati di fallback che possono divergere; non dimostra modelli obsoleti o download sbagliati.

**Proposta:** usare il catalogo già ottenuto tramite servizio/store oppure generare un fallback dal catalogo autorevole; rendere esplicito lo stato «catalogo non disponibile» se si decide di eliminarlo.

**Verifica:** IPC riuscito, fallito e tardivo; modello installato, suggerimento predefinito e progressione download.

### A15 — Ricompressione dei media APKG durante Refine

**Evidenza:** `core/srt-apkg/src/lib.rs:15` e `:125` selezionano Stored in base all'estensione. Gli APKG Vesta contengono media con nomi numerici (`0`, `1`, …), come mostrato anche dalle fixture in `lib/srt-refine/src/cards.rs`. Dopo estrazione e modifica, tali file non hanno estensione e ricadono in Deflate nel repack generico.

**Impatto:** CPU e tempo aggiuntivi per media già compressi, senza presumere una perdita di correttezza. La documentazione della funzione che promette Stored per media precompressi non copre adeguatamente questo caso.

**Proposta:** rendere il repack consapevole del manifest `media`, oppure usare un percorso specifico per pacchetti Anki mantenendo l'helper ZIP generico.

**Verifica:** metodo compressione degli entry numerati, contenuto identico, manifest, metadati SQLite, dimensione e tempo su APKG grandi. L'estensione del nome logico deve guidare la scelta; non trattare ogni file senza estensione come media.

### A16 — I/O temporaneo nel merge APKG

**Evidenza:** `lib/srt-flashcards/src/merge_apkg.rs:35–146`. Ogni database sorgente viene scritto in un file; il primo viene poi copiato nel database finale. Ogni media viene materializzato prima della verifica del nome duplicato e, se duplicato, riletto tramite `files_equal`. La pubblicazione finale usa già streaming e media Stored.

**Impatto:** copie e I/O aggiuntivi su merge grandi o molti media condivisi; non un problema dimostrato di memoria e non un motivo per perdere la verifica byte-per-byte dei conflitti.

**Proposta:** valutare copia diretta del primo database, confronto streaming per duplicati e gestione dei temporanei non necessari. Un hash può ridurre confronti ma richiede una politica di collisione chiara.

**Verifica:** media identici e conflittuali, ID e metadati, fallimento a metà merge, output precedente intatto e uso massimo dello storage temporaneo. Misurare prima su serie reali.

### A17 — CI con setup desktop duplicato

**Evidenza:** `.github/workflows/ci.yml`: job `clippy` e `test` ripetono setup Node/Rust, installazione librerie desktop/Vulkan, icone, `npm ci` e build frontend. Il job frontend compila anch'esso gli asset.

**Tradeoff:** duplicazione YAML e tempo totale dei runner; separazione utile per parallelismo e diagnostica. Non si può dedurre una riduzione del tempo di attesa accorpando i job.

**Proposta:** estrarre setup in composite action mantenendo job distinti; valutare artefatto frontend condiviso solo dopo misure del percorso critico, download e cache. I profili headless e desktop vanno preservati separati.

**Verifica:** cache fredde/calde, durata totale runner e wall time, prerequisiti di `generate_context!`, feature GPU e condizioni che bloccano le release.

### A18 — Duplicazioni negli strumenti di supporto e nei test

**Evidenza:** handshake, segnale SIGINT e attese ripetuti in `cli/srt-translate-cli/tests/cli.rs:202`, `cli/srt-transcribe-cli/tests/cli.rs:102`, `cli/srt-autosync-cli/tests/cli.rs:118`; lista lingue e helper JSON ripetuti in `apps/srt-gui/scripts/merge_translations.py:40` e `split_translations.py:40`.

**Proposta:** helper di test condivisi solo per lifetime/processi e timeout; piccolo modulo comune per cataloghi e I/O degli script se entrambi restano mantenuti. Le asserzioni specifiche del dominio devono restare vicine al test.

**Impatto:** soprattutto manutenzione. Non cancellare test ripetitivi che verificano contratti pubblici differenti; la duplicazione di fixture è meno urgente di quella della logica applicativa.

## 5. Elementi legacy e ridondanze da non eliminare automaticamente

- `lib/srt-translate/src/lib.rs:167`: `get_missing_subtitle_ids` è deprecata e non ha chiamanti interni trovati, ma è pubblica e conserva una semantica distinta (solo ID mancanti). Rimozione solo secondo la policy di compatibilità.
- `lib/srt-flashcards/src/types.rs:159`: `use_timings_from` è dichiaratamente un campo di compatibilità; la generazione usa i timing target. Non è una funzionalità dimenticata da riattivare o un campo eliminabile senza migrazione.
- `lib/srt-translate/src/pool.rs:118`, `lib/srt-transcribe/src/cloud.rs:41`: wrapper legacy di costruzione client convivono con varianti fallibili. Possibile debito API, non duplicazione accidentale. Per nuove chiamate preferire gli errori espliciti; non rompere i consumer.
- `lib/srt-refine/src/cards.rs:140`: campi Anki con `allow(dead_code)` fanno parte di una struttura di deserializzazione. Non dimostrano un ramo esecutivo morto; l'eventuale semplificazione ha beneficio modesto.
- `apps/srt-gui/package.json`: `happy-dom` e `jsdom` sono entrambi usati da test espliciti; non classificare uno dei due come dipendenza inutilizzata senza migrare e verificare quei test.
- `apps/srt-gui/vite.config.ts`: il fallback per CSS virtuale ha una regressione dedicata; non rimuoverlo come workaround obsoleto senza riprodurre il caso freddo e controllare il plugin installato.
- `benchmarking/` e `benchmarking_against_subs2srs/`: strumenti e fixture di riproducibilità, non scarti perché non importati dall'app.
- `vendor/whisper-rs-sys-0.15.0`: patch attiva nel workspace; il nome diverso da `whisper-rs 0.16` non prova obsolescenza.
- `cargo tree --duplicates` mostra famiglie di versioni transitive multiple, per esempio base64 e bitflags. Non si possono unificare forzando una dipendenza diretta se i consumer richiedono API incompatibili.
- I warning GTK/GLib e D-Bus documentati in MAINTENANCE richiedono revisione upstream e audit aggiornato. Questo report non li dichiara risolti né conferma che lo stato online sia invariato.
- `languages.ts`, `languageSearchIndex.ts` e `i18n/index.ts` sovrappongono metadati, ma rappresentano repertori diversi (lingue contenuto, ricerca localizzata e lingue UI). Possibile fonte comune generata, da progettare preservando tali insiemi; nessuna eliminazione suggerita sulla sola somiglianza.

Non è emersa, nelle parti approfondite, una grande sottostruttura interna chiaramente abbandonata e sicuramente eliminabile. Questo non equivale a provare che tutto il codice sia raggiungibile.

## 6. Ordine proposto per la revisione

| Fase | Lavoro da valutare | Rischio | Condizione prima dell'applicazione |
|---|---|---|---|
| 1 | A01–A04: discovery e identità cache | medio | riproduzioni deterministiche e contratto cache concordato |
| 2 | A06: import/handler inutilizzati | basso | ricerca riferimenti e check frontend |
| 3 | A05, A07, A14: normalizzazione e fonti dati | basso/medio | fixture compatibilità, default e conteggio richieste |
| 4 | A08–A09: ricerca Refine | medio | misura heap e latenza su dataset grandi |
| 5 | A12–A13: separazione viste | medio | estrazioni piccole con test montati |
| 6 | A10–A11, A15–A16: prestazioni Rust e archivi | medio/alto | benchmark prima/dopo e invarianti di correttezza |
| 7 | A17–A18: CI e strumenti | basso/medio | misura costi CI e mantenimento della riproducibilità |

Non sono fornite stime orarie o percentuali di accelerazione: mancano baseline rappresentative per sostenerle. La build misurata produce circa 216,47 kB per il chunk Flashcards e 183,21 kB per Settings, prima di gzip; questo aiuta a scegliere cosa profilare, ma non misura il tempo di interazione. I cataloghi lingua sono già suddivisi in chunk: non viene proposta una correzione di caricamento globale non riscontrata.

## 7. Criteri di accettazione per eventuali interventi

1. Ogni modifica deve riferirsi a un rilievo e preservare il contratto pubblico pertinente.
2. Correzioni asincrone: prova del caso precedente e regressione per risposte obsolete, cambio credenziale e teardown.
3. Deduplicazioni: stesso comportamento su configurazioni vecchie, default, eventi e messaggi localizzati.
4. Ottimizzazioni: stessi input e impostazioni, stesso hardware/profile/cache; confrontare tempi, memoria e output, non soltanto il numero di righe rimosse.
5. Persistenza/media: output precedente preservato su errore e cancellazione; validità ZIP/SQLite e riferimenti media verificati.
6. Prima di integrare: gate appropriati e controlli workspace richiesti dal repository; smoke nativi separati per quanto non coperto dai mock.

**Decisioni lasciate aperte:** politica e identità della cache modelli; destino dei campi STT inattivi; budget di memoria Refine; eventuale evoluzione della durability dei checkpoint; convenienza della condivisione degli artefatti CI. Il presente audit non applica né approva queste scelte.
