# Vesta: stato del progetto e priorità — 15 ottobre 2026

## Valutazione

La struttura del prodotto è già solida: 3 crate fondamentali, 8 librerie di
funzionalità e 7 CLI, con il desktop Tauri/Svelte come consumatore. I manifest
di `core/`, `lib/` e `cli/` non dipendono da Tauri. Traduzione, trascrizione,
sincronizzazione e generazione non vanno riscritti nel frontend né in nuove CLI.
La separazione tra elaborazione Rust e orchestrazione Svelte merita di restare.

Il debito più visibile è nella composizione del frontend. Al checkpoint
`2d499f7`, `FlashcardsTab.svelte` conta 2.713 righe; Translate 1.513, Sync 1.479,
Settings 1.408, Transcribe 1.371 e Refine 1.195. Sono indicatori di responsabilità
accumulate, non soglie automatiche per dividere file. Le tab contengono ancora
preferenze, discovery, gestione eventi, azioni e presentazione insieme.

Esistono già `PathPickerField`, `FooterActions`, pannelli media, selettori,
`asyncControl`, validazione dei percorsi e moduli `workflows/`. Prima di creare
un componente condiviso bisogna verificare questi punti di riuso. La prossima
estrazione dovrebbe togliere una responsabilità completa da una tab e darle un
contratto testabile; spostare soltanto markup non basta.

## Piano e criteri di completamento

| Priorità | Intervento | Criterio verificabile |
|---|---|---|
| Immediata | Correggere discovery obsoleta nelle impostazioni e cancellazione/uscita delle CLI | Risposte superate non aggiornano modelli/preferenze; cancellazione non viene interpretata come successo; regressioni offline. |
| Successiva | Estrarre lo stato/provider workflow da Settings | La tab compone pannelli; endpoint, scelta modello e persistenza si verificano senza montare tutta la schermata. |
| Successiva | Estrarre un flusso autonomo da Flashcards | Separare coordinamento serie/preview/export secondo responsabilità reali, preservando ownership, stop/riprova e impostazioni del run. |
| Successiva | Coprire le interazioni Svelte più rischiose | Test con componenti montati per cambio scheda, endpoint durante debounce, stop/riprova, errore salvataggio e dialogo chiuso durante una richiesta. |
| Successiva | Allineare i contratti CLI dove servono agli script | Matrice esplicita di stdout, stderr, exit code, cancellazione e pubblicazione; test subprocess sui binari, senza aggiungere flag JSON inutilizzati ovunque. |
| Successiva | Rendere atomici i salvataggi SRT ancora diretti | `SrtParser::save_file` usa `fs::write`: validare prima protegge da input invalido, ma non da disco pieno/interruzione durante la scrittura. Usare file temporaneo nella destinazione e sostituzione, con prove di errore e compatibilità piattaforma. |
| Prima di distribuire | Eseguire la matrice dei pacchetti installati | Linux/Windows: setup e riavvio, playback/seek, associazioni file, importazione Anki e GPU/fallback verificati sulle piattaforme. |

Non serve un contenitore generico per tutte le operazioni: `AsyncOperation`
gestisce il lock, `LatestRequest` la validità dei risultati, e i workflow
gestiscono la sequenza specifica. Unificare queste responsabilità rende più
difficile verificare la cancellazione. Anche i footer e i picker possono avere
azioni diverse pur avendo lo stesso aspetto.

## Delega e revisione

Due worker `gpt-6-luna`, entrambi con contesto nuovo, hanno ricevuto ownership
separata: frontend e CLI. Il coordinatore mantiene analisi, documenti, verifica
incrociata e integrazione. Il progresso preesistente è stato salvato su `dev`
nel commit `2d499f7` prima degli interventi.

La discovery Settings aveva due contatori manuali. Invalidavano le risposte
solo all'avvio della richiesta successiva, lasciando scoperti cleanup e debounce.
Il workflow `LatestModelDiscovery` riusa `LatestRequest` e restituisce `null`
per successi o errori superati. Le due discovery restano indipendenti; il cleanup
invalida i risultati anche quando non parte subito un'altra richiesta.
La revisione ha richiesto anche i casi di successo tardivo senza nuova richiesta
e riapertura dello stesso endpoint mentre fallisce il vecchio controllo.

Nelle CLI, `srt-refine` è già un buon riferimento per output strutturato,
cancellazione e pubblicazione completa. `srt-autosync` invece restituiva successo
quando l'analisi era cancellata, installava Ctrl+C dopo il download del modello
e scartava gli errori di applicazione degli anchor. Questi sono problemi di
contratto concreti da correggere prima di espandere l'interfaccia dei comandi.
La revisione ha aggiunto un test subprocess SIGINT con modello stub, cache
isolata, handshake del probe e timeout: verifica direttamente exit 130,
stdout vuoto e output esistente intatto. Un'altra regressione applica prima
un'ancora valida e poi una invalida: il risultato parziale non viene scritto.
Il token viene ricontrollato prima del salvataggio sincrono, che una volta
iniziato non è interrompibile in sicurezza.

| CLI | Stato e prossimo controllo utile |
|---|---|
| `srt-extract` | Test subprocess per formati, JSON e fallimenti; resta utile la pubblicazione atomica degli output scritti direttamente. |
| `srt-sync` | Anchor e range validati prima del salvataggio, test di drift e preservazione; condivide il salvataggio SRT diretto. |
| `srt-refine` | Ispezione/annotazione, pubblicazione completa per default, parziale esplicita, loopback e SIGINT già coperti. |
| `srt-autosync` | Corretto il contratto cancellazione/download/anchor in questo passaggio; richiede ancora smoke con Whisper reale per la qualità dell'allineamento. |
| `srt-transcribe` | Protocollo `--json` esplicito e pipeline condivisa; priorità a test subprocess del JSON finale e del codice di errore, oltre ai test del motore. |
| `srt-translate` | Errori di configurazione sanitizzati e provider testati nel motore; aggiungere regressioni subprocess per resume, interruzione e output. |
| `srt-flashcards` | Motore condiviso e smoke APKG reale; estendere i casi CLI su input/flag invalidi e preservazione dei file. |

## Test: dove aggiungerli

La suite Vitest gira in Node con trasporto nativo simulato: copre utility,
store, servizi e workflow, ma non dimostra focus, tastiera, accessibilità,
layout o il collegamento effettivo dei cleanup di un componente montato.
Il test dei comandi IPC controlla la registrazione dei nomi letterali, non
gli schemi degli argomenti o tutti i comandi costruiti dinamicamente.

Le nuove regressioni devono proteggere risultati osservabili: risposta vecchia
che arriva dopo un cambio endpoint, errore vecchio dopo chiusura/riapertura,
JSON leggibile dallo script e file esistente preservato dopo fallimento.
I casi nativi hanno bisogno di FFmpeg reale o processi/HTTP locali controllati;
una promessa IPC simulata non certifica il comportamento del processo figlio.

## Documentazione

`ARCHITECTURE.md` descrive confini e ownership; `QUALITY.md` mantiene i comandi
e la matrice di verifica; README e guide modulo spiegano utilizzo e integrazione.
Gli audit datati conservano le prove storiche: non devono diventare specifiche
concorrenti. Gli audit precedenti ora rimandano a questa valutazione; alcuni
limiti storici, fra cui snackbar a messaggio singolo e lacune di localizzazione,
sono già superati dal checkpoint.

La guida moduli ora precisa anche che copiare una sola directory non crea
automaticamente un crate autonomo: manifest e dipendenze ereditano il workspace;
Whisper richiede inoltre patch e toolchain native.

## Evidenza della baseline

Sul checkout locale Linux, prima delle modifiche dei worker:

- `python3 build-scripts/quality_check.py --desktop --smoke`: passato,
  270 test frontend in 50 file e 285 test Rust (esclusa `whisper-bench`).
- Svelte: zero errori e warning; build produzione e regressione CSS a freddo passate.
- Localizzazione: 1.203 chiavi per ciascuno dei 15 cataloghi, zero errori;
  6 segnalazioni euristiche su nomi/formati rimangono da distinguere dagli errori bloccanti.
- Smoke: 3 esportazioni, ciascuna con 10 note e 20 media non vuoti,
  integrità ZIP e SQLite verificata. Mediana debug circa 0,509 s per export;
  è un controllo sintetico locale, non una nuova rivendicazione di prestazioni.
- Clippy workspace con warning come errori e 5 test degli hook Git: passati.

## Verifica finale del coordinatore

Gli interventi sono salvati su `dev` separatamente: `341df9a` per la discovery
frontend e `2e59e15` per il contratto della CLI. Dopo la revisione del codice e
il rafforzamento dei test richiesto ai worker:

- Gate qualità desktop/smoke passato con 274 test frontend in 51 file,
  build, CSS, localizzazione e tre esportazioni APKG; questo run precede
  l'ultimo rafforzamento dei soli test e del salvataggio autosync.
- `cargo test --workspace` rieseguito sul codice CLI finale: 290 test passati,
  inclusi desktop, benchmark app e subprocess SIGINT offline.
- `cargo clippy --workspace --all-targets -- -D warnings` rieseguito sul codice
  finale: passato. Formatting e controllo whitespace passati.
- Link locali dei documenti modificati verificati; gli audit storici restano
  consultabili e rimandano alla valutazione corrente.

Non è stato eseguito push né pubblicata una release.

Le verifiche automatizzate locali non certificano installer, WebKit/GStreamer,
Anki reale, servizi cloud autenticati o altre piattaforme. Per questi casi resta
valida la [matrice di release](../QUALITY.md#release-smoke-matrix).

## Secondo ciclo: implementazione delle priorità

Su richiesta dell'utente, il lavoro è proseguito oltre la prima correzione.
I commit rimangono in inglese, con tipi Conventional Commits riconosciuti dal
preset `conventionalcommits` di `.releaserc`. I due worker Luna hanno mantenuto
ownership separate; il coordinatore ha implementato la persistenza, revisionato
i cambiamenti e integrato le verifiche.

### Persistere senza troncare il file precedente

`srt-parser::persistence` centralizza la pubblicazione tramite temporaneo nella
stessa directory. Il callback termina la scrittura, il file viene sincronizzato
e poi sostituito. Parser SRT, trascrizione, sessioni sync, TSV/APKG flashcard e
output CLI extract usano questo percorso. Transcribe usa anche un writer
bufferizzato e ricontrolla la cancellazione prima della pubblicazione.

Le regressioni simulano errori dopo scritture parziali, fallimenti nello
streaming APKG, destinazioni read-only, symlink e hard link. Verificano i byte
del file precedente e la rimozione dei temporanei. Un controllo condiviso di
identità file impedisce a extract/translate di usare il proprio input come
destinazione, anche attraverso alias; `same-file` era già presente nel lock e
fornisce il confronto portabile anche oltre Unix.

Questo protegge il singolo file pubblicato. La directory media di un export
TSV non è ancora transazionale; ownership/ACL estese, coordinamento tra processi
e durabilità della directory dopo perdita di alimentazione restano fuori dal
contratto. Una pubblicazione sincrona già iniziata non viene interrotta da Ctrl+C.

### Settings e discovery Flashcards

`DefaultLlmProviderSettings` estrae le regole provider/modello, accesso,
persistenza, reset e discovery da SettingsTab. Mantiene distinti i due canali
di discovery e invalida subito i risultati al cambio provider. I test coprono
fallback di accesso, preferenze persistite, risultati obsoleti e scelta manuale
del modello mentre una discovery è ancora in corso. La tab conserva rendering,
traduzioni e lifecycle del polling.

Flashcards avvia insieme i lookup indipendenti per sottotitolo companion e media.
Il pool di quattro file produce al massimo otto IPC simultanee: i test verificano
il limite, l'ordine dei risultati e la conservazione del risultato valido quando
l'altro lookup fallisce. È una riduzione dell'attesa sequenziale, senza una
nuova rivendicazione di velocità end-to-end.

### Contratti delle CLI

- Transcribe JSON restituisce un record terminale leggibile e status 1 sugli
  errori; su cancellazione prima della pubblicazione restituisce `cancelled:true`
  e status 130, senza il messaggio Ctrl+C su stderr. Il ritorno normale dal
  runtime sostituisce `process::exit`, consentendo il cleanup delle risorse.
- Translate propaga Ctrl+C al motore, restituisce 130 e conserva l'ultimo
  checkpoint completato. Il checker `--check-missing` restituisce 1 quando
  trova righe mancanti o incoerenti, anziché segnalare successo agli script.
- Extract pubblica gli output atomicamente e rifiuta gli alias dell'input.

I test subprocess usano errori locali, processi simulati o HTTP loopback.
I test SIGINT attendono un handshake prima di inviare il segnale, con timeout
e cleanup del processo; non richiedono modelli scaricati o API a pagamento.

### Lavoro successivo consigliato

Verifiche finali del secondo ciclo: 281 test frontend in 52 file, Svelte senza
errori/warning, build e CSS a freddo passati, 15 cataloghi senza errori bloccanti
(restano le 6 segnalazioni euristiche già note). Il gate desktop/smoke è passato
con tre APKG validi da 10 note/20 media ciascuno. L'intero workspace Rust conta
306 test passati, inclusa `whisper-bench`; Clippy con `-D warnings`, formatting
e controllo whitespace sono passati. La revisione indipendente dei writer non
ha trovato difetti di pubblicazione; ha richiesto di chiarire il limite di
Ctrl+C durante il commit sincrono, ora documentato nella CLI.

1. Aggiungere test con componenti Svelte montati: il nuovo controller è testato,
   ma focus, tastiera e collegamento dei cleanup richiedono prove dell'interfaccia.
2. Estrarre da FlashcardsTab il prossimo flusso autonomo di preview/serie/export,
   con snapshot delle impostazioni e test stop/riprova. La parallelizzazione
   della discovery non completa questa separazione.
3. Gestire come transazione l'export TSV con la sua directory media, oppure
   generare versioni separate e pubblicare un riferimento finale. La sola
   sostituzione del TSV non protegge i media precedenti durante una rigenerazione.
4. Eseguire la matrice dei pacchetti installati su Linux/Windows e le prove Anki,
   GPU/fallback e servizi autenticati. I test offline non certificano tali percorsi.
5. Valutare keychain di sistema per le credenziali ed errori strutturati negli
   altri confini nativi, in cambiamenti dedicati con migrazione compatibile.
