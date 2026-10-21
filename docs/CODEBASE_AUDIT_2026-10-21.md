# Audit del codebase Vesta — 21 ottobre 2026

## Esito e perimetro

Il debito più concreto riguarda I/O ripetuto nella traduzione, copie di contratti/default fra frontend e backend e gestione di alcune operazioni ancora sincrone in funzioni async. Non emerge una ragione sufficiente per accorpare indiscriminatamente crate, eliminare CLI autonome o rimuovere API pubbliche compatibili.

Questo documento è destinato alla discussione **prima di modificare implementazione o comportamento**. Le proposte sotto non sono decisioni approvate. Durante questo audit non è stato modificato codice applicativo: è stato aggiunto soltanto questo report; i comandi di verifica hanno rigenerato artefatti locali di build.

Base Git: `d4faa69a5efbac6f138522c4add0fe44a999dd61`, con numerose modifiche locali preesistenti, inclusi file non tracciati. L'oggetto dell'analisi è il checkout corrente, non il solo commit. I riferimenti di riga valgono per questa fotografia e possono cambiare. Consultata anche `docs/AUDIT_REVIEW_2026-10-21.md`, trattandola come contesto da verificare e non come prova automatica.

### Metodo e limiti

- Inventario di manifest, CI, dimensioni dei file e organizzazione core/lib/CLI/GUI; ricerca mirata dei chiamanti, API deprecate, parsing, processi e I/O.
- Scansione di blocchi identici di 16 righe, dopo rimozione dell'indentazione, nei sorgenti Rust/TypeScript/Svelte. Verifica manuale dei risultati più significativi; SVG, classi CSS e markup ripetuti non sono automaticamente difetti.
- Il campione della scansione comprende 292 file e 62.220 righe, escludendo file con “test” nel nome ma includendo alcune fixture. Non è una misura del solo codice di produzione. L'inventario più ampio core/lib/CLI/GUI comprende circa 89.447 righe, inclusi test e cataloghi JSON.
- Lettura mirata, non revisione manuale riga per riga di tutto il repository. Vendor Whisper/ggml escluso dal giudizio sul codice proprietario; benchmark e script esaminati a livello di struttura, senza verificarne ogni algoritmo.
- Nessun profiling end-to-end, test con provider a pagamento, prova hardware GPU, verifica Anki installato o matrice desktop multipiattaforma. Nessuna verifica online delle versioni più recenti o del ritiro di modelli: “divergente” sotto non significa “non più supportato”.
- Ricerca per riferimenti e build non costituiscono una prova formale di assenza di codice morto. API Rust pubbliche e invocazioni dinamiche richiedono particolare cautela.

Priorità: **P2** = intervento circoscritto da valutare nella prossima manutenzione; **P3** = miglioramento subordinato a misure o beneficio modesto. Nessun P0/P1 dimostrato in questo perimetro. Confidenza descrive l'evidenza del fenomeno, non il guadagno prestazionale.

## Quadro dei rilievi

| ID | Tema | Priorità | Confidenza | Natura |
|---|---|---|---|---|
| A01 | Anteprima traduzione: parsing completo ogni 2 secondi | P2 | Alta | I/O e CPU evitabili |
| A02 | Checkpoint completo per batch, sotto lock | P2 | Alta | Costo crescente, durabilità da preservare |
| A03 | Letture/preparazione sincrone nei comandi async di traduzione | P2 | Alta | Occupazione worker Tokio |
| A04 | Template e CSS Anki duplicati fra Rust e TypeScript | P2 | Alta | Rischio di divergenza |
| A05 | Due default diversi per il modello Google | P2 | Alta | Incoerenza di contratto |
| A06 | Controller di anteprima audio ripetuto | P2 | Alta | Manutenzione del ciclo di vita |
| A07 | Chunk cloud letti con I/O sincrono | P2 | Alta | Occupazione worker Tokio |
| A08 | Pipeline cloud sequenziale e preparazione anticipata | P3 | Alta | Potenziale latenza, da misurare |
| A09 | Retry HTTP e wrapper provider ripetuti | P3 | Alta | Manutenzione e coerenza |
| A10 | API desktop deprecate ancora utilizzate | P3 | Alta | Debito di compatibilità |
| A11 | Tab GUI molto estese | P3 | Alta | Complessità di manutenzione |
| A12 | Setup CI desktop ripetuto e test sovrapposti | P3 | Alta | Costo CI da misurare |
| A13 | Helper minimi e interfaccia setup ripetuti | P3 | Alta | Pulizia a basso rendimento |
| A14 | Ricerca lineare dell'ID in SyncEngine | P3 | Alta | Micro-ottimizzazione condizionata |
| A15 | Classificazione legacy degli errori via stringhe | P3 | Alta | Compatibilità esplicita |
| A16 | Fallback silenzioso nella costruzione client HTTP | P3 | Alta | Configurazione degradata poco visibile |

## Analisi dettagliata

### A01 — Anteprima della traduzione

**Evidenze:** `apps/srt-gui/src/lib/tabs/TranslateTab.svelte:631-650` richiama l'anteprima ogni 2.000 ms. `apps/srt-gui/src-tauri/src/commands/translate.rs:341-393` rilegge e riparsa input e output interi, ordina tutti gli ID tradotti e restituisce soltanto gli ultimi N (la GUI chiede 10).

**Impatto:** lavoro ripetuto anche se l'output non è cambiato. Per refresh: parsing proporzionale alle dimensioni dei due file e ordinamento O(T log T). La frequenza è dimostrata; il costo percepito su file normali non è misurato. Il frontend possiede già protezione dalle risposte obsolete e una richiesta pendente condivisa: non risulta corretto descrivere questo percorso come polling senza controllo delle sovrapposizioni.

**Opzioni da discutere:** conservare l'input normalizzato per la durata del job; evitare parsing se la revisione dell'output non cambia; emettere le ultime coppie quando un checkpoint viene pubblicato. Una cache deve considerare cambio file, modifiche esterne, cancellazione e ordine degli ID.

**Validazione futura:** contare letture e tempi su 1.000/10.000/50.000 sottotitoli; confrontare anteprima e output in resume/cancel e dopo cambio file. L'eliminazione di letture inutili può essere valutata senza ridurre la frequenza dei checkpoint.

### A02 — Checkpoint della traduzione

**Evidenze:** `lib/srt-translate/src/lib.rs:676-684`, `891-895`, `927-938`: aggiornamento della mappa e salvataggio completo con `OwnedMutexGuard`, spostato su `spawn_blocking`. Il guard resta posseduto durante la scrittura per evitare pubblicazioni fuori ordine.

**Impatto:** il blocco dell'executor è già corretto, ma non il volume totale delle scritture. Con B batch di dimensione costante e traduzioni accumulate, la somma dei record riscritti cresce quadraticamente in B nel caso ordinario senza resume. Sort/serializzazione e I/O sotto lock possono limitare i worker concorrenti. Nessuna misura dimostra che dominino le chiamate LLM.

**Opzioni:** writer unico con versioni e snapshot; journal incrementale con compattazione; eventuale accorpamento dei salvataggi soltanto dopo aver stabilito quanta perdita di lavoro al crash sia accettabile. Anche liberare il lock prima dell'I/O richiede gestione esplicita dell'ordine.

**Rischio:** alto rispetto a un semplice refactoring, perché il comportamento attuale garantisce durabilità per batch. Test esistente `concurrent_checkpoints_preserve_all_completed_batches` da conservare; aggiungere crash/failure e ordine con writer lento se si cambia il protocollo. Misurare byte scritti, attesa lock e throughput prima di scegliere.

### A03 — Preparazione sincrona nelle funzioni async di traduzione

**Evidenze:** `commands/translate.rs:126` esegue `SrtParser::parse_file` dentro `suggest_translation_context`; `:190` chiama `load_translation_source`, che legge/parsa/normalizza sincronicamente a `:311-322`. Anche l'anteprima A01 usa parsing sincrono nella funzione async.

**Impatto:** questi segmenti occupano un worker Tokio fino al completamento. Non dimostrano automaticamente un blocco del thread grafico, ma su file grandi o disco lento possono interferire con altre operazioni asincrone.

**Proposta:** spostare la preparazione completa su `spawn_blocking`, con dati posseduti, oppure adottare letture async seguite da parsing in un worker. Evitare di distribuire molti piccoli task per ogni riga. Validare errori, normalizzazione, input/output alias e cancellazione durante preparazione.

### A04 — Template e stile Anki in due linguaggi

**Evidenze:** `lib/srt-flashcards/src/export_apkg.rs:288-321` e `apps/srt-gui/src/lib/types/noteTypes.ts:345-369` contengono HTML e CSS corrispondenti; i commenti chiedono espressamente di mantenerli sincronizzati. La scansione rileva blocchi CSS identici.

**Impatto:** una modifica può raggiungere solo GUI o fallback CLI. I test Rust presenti verificano l'inserimento dei font nei due percorsi, usando però un CSS GUI sintetico: non equivalgono a una verifica di parità completa fra le due copie reali.

**Proposta:** asset HTML/CSS canonici condivisi tramite import raw e `include_str!`, mantenendo distinta l'iniezione dello script delle tag e del dizionario; in alternativa test automatico di parità delle parti che devono coincidere. Verificare personalizzazioni salvate, reset e pacchetti generati dai due ingressi. Non sostituire template personalizzati già salvati.

### A05 — Default Google divergenti

**Evidenze:** `lib/srt-translate/src/translator.rs:38-45` usa `gemini-2.0-flash` in `TranslatorConfig::default`; `lib/srt-translate/src/pool.rs:29-35` usa `gemini-2.5-flash` nel default Google. Anche il catalogo GUI contiene il secondo modello.

**Impatto:** un consumer diretto del costruttore predefinito può avere un comportamento diverso dal pool. Questo è un default pubblico divergente, non prova che il modello precedente sia ritirato o che la GUI lo usi.

**Proposta:** decidere il contratto dei default e riutilizzare una costante o funzione unica; coprire il confronto con un test. Non aggiornare modelli scelti esplicitamente né i nomi nelle fixture storiche senza motivo.

### A06 — Gestione dell'anteprima audio duplicata

**Evidenze:** `apps/srt-gui/src/lib/modals/EpisodeMediaSettingsModal.svelte:35-108` e `apps/srt-gui/src/lib/panels/AudioClipsPanel.svelte:314-402` ripetono richiesta dei dati server, generazione preview, URL con token, creazione `Audio`, callback, playback e invalidazione. Sono presenti differenze legittime nella scelta del segmento e degli override.

**Impatto:** una correzione di race, errore o teardown deve essere replicata. La protezione `LatestRequest` è già presente in entrambi: non è un bug dimostrato di risposta obsoleta. Nel `close` del modal, `pause` e assegnazione a `false` sono ripetuti dopo `stopAudio`.

**Proposta:** controller audio condiviso per trasporto/playback/cleanup, lasciando ai chiamanti la costruzione dei parametri. Evitare un componente unico che inglobi pannello e modal. Verificare cambio episodio, doppio clic, chiusura durante generazione e rifiuto di `play()`; conservare le prove montate esistenti.

### A07 — Lettura dei chunk cloud sul worker async

**Evidenza:** `lib/srt-transcribe/src/pipeline.rs:458` usa `std::fs::read(chunk_path)` all'interno di `run_cloud`, prima del `tokio::select!` che osserva cancellazione e richiesta HTTP. I chunk sono di 480 secondi, WAV mono 16 kHz PCM 16 bit (`audio.rs`): circa 15,36 MB di campioni per chunk pieno, oltre all'header.

**Impatto:** I/O sincrono e lettura completa prima del controllo di cancellazione della richiesta. La memoria è limitata al chunk nel ciclo corrente, non all'intero film caricato in RAM.

**Proposta:** `tokio::fs::read` con cancellazione al confine dell'operazione; valutare streaming soltanto se supportato uniformemente dagli adapter. Verificare file incompleto/assente, disco lento, cancellazione e offset. Non aumentare la concorrenza per risolvere il solo problema di I/O sincrono.

### A08 — Trascrizione cloud strettamente sequenziale

**Evidenze:** `lib/srt-transcribe/src/pipeline.rs:414-423` prepara i chunk prima dell'invio; `:446-480` trascrive un chunk alla volta. Il client HTTP viene riutilizzato correttamente.

**Impatto potenziale:** latenza seriale per media lunghi e ritardo prima della prima richiesta. Una pipeline con preparazione/invio sovrapposti o parallelismo limitato potrebbe aiutare. Il comportamento corrente limita memoria e pressione sulle quote: non è di per sé un errore.

**Decisione necessaria:** budget di concorrenza per provider, ordinamento dei progressi e comportamento al primo errore. Misurare velocità, RAM, file temporanei e rate limit con provider reale autorizzato prima di intervenire. Non promettere un fattore di accelerazione derivato soltanto dal numero di worker.

### A09 — Wrapper e retry duplicati per provider

**Evidenze:** `lib/srt-translate/src/translator.rs`, funzioni `translate_batch_google`/`translate_batch_openai`, wrapper di prompt singolo e contestuale, cicli `call_google_api`/`call_openai_api`. Il flusso send/status/body/retry/decode si ripete; `retry_backoff` e diagnostica sono già condivisi.

**Impatto:** manutenzione ripetuta, non necessariamente costo runtime significativo. Formati di autenticazione, payload e risposta sono diversi; mantenere adapter distinti è ragionevole.

**Proposta:** condividere costruzione prompt e parsing batch sopra al dispatcher, poi valutare un piccolo helper per il trasporto. Preservare differenze di trimming e comportamento d'errore. I test provider esistenti sono una base utile. Non introdurre un framework di provider per risparmiare poche righe.

### A10 — API deprecate desktop

**Evidenze:** `apps/srt-gui/src-tauri/src/commands/extract.rs:47` consente l'uso deprecato di `app.shell().open`; `main.rs:154` contiene un blocco `allow(deprecated)` per policy di navigazione WebKit Linux.

**Impatto:** dipendenza da percorsi che richiederanno manutenzione. Non sono codice inutilizzato né prova di malfunzionamento attuale.

**Proposta:** migrazioni separate, con verifica del comportamento di apertura cartelle/file e dei link nel webview reale. Leggere la documentazione della versione effettiva prima di scegliere il sostituto; questo audit non ha effettuato una valutazione online delle API alternative.

### A11 — Tab GUI con responsabilità concentrate

**Evidenze (righe totali):** Flashcards 1.801; Translate 1.514; Sync 1.479; Transcribe 1.399; Settings 1.358; Refine 1.299. La struttura contiene già servizi, store, workflow e componenti condivisi; non parte da un monolite privo di separazione.

**Impatto:** costo cognitivo, modifiche che intrecciano lifecycle/IPC/rendering e maggiore superficie da verificare. Il numero di righe non prova lentezza del rendering.

**Proposta:** estrarre responsabilità solo quando hanno un confine concreto, ad esempio A06 o il controller dell'anteprima tradotta. Conservare ownership di cancellazione e cleanup. La build conferma chunk separati: Flashcards ~214,54 kB, Settings ~179,43 kB non compressi. Misurare avvio e mount prima di imporre ulteriore suddivisione; lazy loading è già implementato.

### A12 — Ripetizione nella CI

**Evidenze:** `.github/workflows/ci.yml`: job clippy/test desktop ripetono installazione pacchetti nativi, SDK Vulkan, Node/npm, generazione icone e build frontend. Headless testa inoltre molti crate già inclusi nella suite workspace.

**Impatto:** duplicazione di manutenzione e potenziale consumo di minuti CI. Sono presenti cache Rust/sccache e job indipendenti: non si può dedurre il costo reale dai soli YAML.

**Proposta:** misurare tempi cold/warm; valutare composite action per il setup. Separatamente valutare artefatti frontend condivisi. Una dipendenza fra job può rallentare il feedback; i test headless garantiscono anche l'indipendenza da feature desktop e non vanno cancellati solo per sovrapposizione nominale.

### A13 — Duplicazioni piccole e markup

**Evidenze:** `isInputActive` in `apps/srt-gui/src/App.svelte:107` e `components/ShortcutOverlay.svelte:36` è identico; `command_succeeds` in `commands/info.rs:40` e `owned_by` in `commands/updates.rs:24` eseguono lo stesso schema subprocess. La scansione rileva anche porzioni del pulsante download FFmpeg in FlashcardsTab/TranscribeTab e overlay drag/drop in SyncTab/TranscribeTab.

**Impatto:** basso; per subprocess i nomi esprimono contesti diversi. Le classi e gli SVG ripetuti non costituiscono un problema prestazionale dimostrato.

**Proposta:** helper condiviso per il riconoscimento del focus editabile quando si interviene sulle shortcut; eventuale card di setup FFmpeg se il flusso deve evolvere in parallelo. Non dedicare una ristrutturazione generale a pochi wrapper.

### A14 — Lookup ID lineare

**Evidenza:** `lib/srt-sync/src/engine.rs:444-451`, `go_to_subtitle`, usa `iter().position`. È O(N). Le ricerche per tempo vicine usano già `partition_point`: non vanno descritte come scansioni lineari complete.

**Impatto:** probabilmente limitato per navigazione manuale; da profilare se chiamato ripetutamente su grandi dataset.

**Proposta condizionata:** indice ID→posizione costruito insieme a `sorted_ids`, solo se la frequenza lo giustifica. Costo aggiuntivo O(N) di memoria e obbligo di mantenere l'indice coerente. Priorità inferiore ad A01/A03/A07.

### A15 — Compatibilità legacy negli errori

**Evidenze:** `lib/srt-translate/src/scheduler.rs:101-115` controlla prima `ProviderError` tipizzato, poi esegue euristiche testuali per integrazioni legacy/custom. `diagnostics.rs:57-87` usa status e indizi del body per classificare gli errori dei provider.

**Impatto:** il fallback può interpretare messaggi ambigui; rimane una compatibilità deliberata e testata. È inesatto dire che tutti gli errori della traduzione siano ancora classificati soltanto via stringhe.

**Proposta:** documentare l'ambito del fallback e raccogliere casi reali; evolvere adapter custom verso errori tipizzati. Non rimuoverlo senza definire compatibilità e migrazione.

### A16 — Fallback nella costruzione dei client

**Evidenze:** `lib/srt-translate/src/translator.rs:58-62` ripiega su `Client::new` se fallisce la configurazione con timeout 10/120 s; `lib/srt-transcribe/src/cloud.rs:33-37` usa `unwrap_or_default` dopo il builder con timeout 900 s.

**Impatto:** il fallback non conserva esplicitamente i timeout richiesti e nasconde l'errore di costruzione. Non è stata riprodotta tale condizione. In traduzione `new` è pubblico e infallibile, quindi modificarne la firma è una scelta di API.

**Proposta:** valutare un costruttore fallibile aggiuntivo mantenendo compatibilità; rendere visibile il fallimento anziché degradare silenziosamente. Testare il contratto della configurazione senza introdurre retry infiniti.

## Candidati esclusi o non dimostrati

- **Componenti Svelte orfani:** una ricerca dei nomi nel sorgente di produzione non ha prodotto candidati evidenti oltre alle fixture dei test. Le fixture hanno una ragione d'essere e non sono proposte per rimozione. La ricerca per nome non è un'analisi completa del grafo degli import.
- **Due ambienti DOM:** happy-dom e jsdom sono entrambi effettivamente selezionati nei test; non sono dipendenze dimostrate inutilizzate. Unificarli richiederebbe controllare le differenze di comportamento.
- **Duplicati transitivi Cargo:** `cargo tree -d` mostra versioni multiple (ad esempio bitflags 1/2), dovute anche a GTK/Tauri e tool di build. Non dimostra che siano eliminabili o tutte presenti nello stesso binario/runtime.
- **Localizzazioni:** 15 cataloghi simmetrici non rappresentano 15 copie inutili di logica. La build emette chunk per lingua e il caricamento è già on demand.
- **API Rust deprecata:** `get_missing_subtitle_ids` è mantenuta compatibile. L'assenza di chiamanti interni non autorizza a rimuoverla da una libreria riutilizzabile.
- **Crate separati, tipi di dominio e main CLI sottili:** la somiglianza strutturale non prova ridondanza. Parsing, allineamento, trascrizione e packaging possono avere invarianti diversi; non è stato dimostrato che unificarli riduca costi o bug.
- **Benchmark, fixture, vecchi nomi di modelli nei test:** non eliminabili in base all'età. Il vendor locale richiede valutazione del motivo della patch prima di aggiornamenti o rimozioni.
- **Correzioni già presenti:** pubblicazione export su worker dedicati, modulo media condiviso, protezione delle preview obsolete, componenti precedentemente rimossi e pulizia del vecchio lockfile non sono nuovi problemi aperti di questo report.
- **Codice obsoleto per versioni esterne:** nessuna dichiarazione di fine supporto è stata verificata. L'evidenza locale più solida è la deprecazione esplicita A10 e il default divergente A05.

## Verifiche eseguite in questo audit

| Verifica | Esito |
|---|---|
| `npm test -- --reporter=dot` | 75 file, 382 test passati |
| `npm run check` | 0 errori, 0 avvisi Svelte |
| `npm run build` | Riuscita, 334 moduli trasformati |
| `cargo test --locked -p srt-parser -p srt-translate -p srt-sync --offline` | 92 test passati: 27 parser, 35 sync, 14 translate unitari, 8 lifecycle, 8 provider |
| Scansione duplicati esatti normalizzati | Eseguita; risultati selezionati verificati manualmente |
| Ricerca riferimenti componenti e deprecazioni | Eseguita, con i limiti dichiarati |
| `cargo tree --locked --offline -d` | Inventario esplorativo delle duplicazioni transitive |

I test frontend producono anche log di errori simulati attesi; i test montati UpdateSettingsCard emettono avvisi runtime di binding non reattivo nel contesto del test. Questo non contraddice lo static check senza avvisi e non dimostra da solo un difetto nello store reale. I processi Node emettono inoltre una deprecazione `module.register()`, da attribuire con una trace prima di cambiare dipendenze.

Non eseguiti: intera suite Rust workspace, Clippy workspace, audit vulnerabilità, verifica aggiornamenti online, benchmark comparativi o test manuali desktop. I test verdi non certificano le prestazioni e non escludono problemi nelle parti non esercitate.

## Sequenza proposta per la discussione

1. Esaminare A01, A03 e A07 come interventi circoscritti: ridurre lavoro ripetuto e I/O sui worker, conservando il comportamento osservabile.
2. Stabilire il default pubblico A05 e la proprietà degli asset A04; scegliere fra fonte unica e controllo di parità.
3. Valutare A06 come estrazione di un controller già duplicato, preservando i due flussi UI.
4. Per A02 e A08 raccogliere misure prima di scegliere architettura, frequenza dei salvataggi o concorrenza. La durabilità per batch non è una variabile puramente tecnica.
5. Inserire A09–A16 nella normale manutenzione solo quando il beneficio supera migrazione e regressioni; evitare un grande refactoring cumulativo.

Per ogni modifica futura: un obiettivo osservabile, baseline riproducibile, verifica di cancellazione/errori e confronto prima/dopo. Nessuna stima percentuale di accelerazione o risparmio è supportata dalle sole evidenze raccolte qui.
