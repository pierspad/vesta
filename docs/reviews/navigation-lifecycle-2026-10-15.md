# Vesta: navigazione e lifecycle delle tab

Baseline su `dev`: `c9ba900`, checkout pulito. Questo ciclo prosegue il
[piano architetturale](architecture-follow-up-2026-10-15.md) con test montati e
correzioni di ownership, senza cambiare i motori Rust.

## Contratto

Le tab caricate restano montate anche quando non sono visibili. Il cambio di
`active` sospende le interazioni della scheda; il lavoro nativo già avviato e i
risultati restano disponibili. Chiudere una preview invalida richieste e playback,
ma conserva gli edit dei sottotitoli utilizzabili dalla generazione.

Un evento tastiera consumato dal suo proprietario non deve essere gestito di
nuovo dalla tab o dall'app. Durante composizione IME ed editing di un campo,
le scorciatoie devono rispettare l'input nativo. Le registrazioni asincrone dei
listener vanno rilasciate anche quando terminano dopo la distruzione del componente.

## Problemi verificati e interventi

- Align registrava scorciatoie sulla finestra senza verificare `active`, quindi
  poteva aprire dialoghi anche da una scheda nascosta. Il medesimo handler era
  collegato sia al pannello sia alla finestra, senza controllo `defaultPrevented`:
  una pressione poteva essere elaborata due volte. Ora controlla visibilità,
  eventi consumati, composizione, dialoghi aperti e campi editabili. I test montano
  Align, cambiano `active`, inviano eventi reali e verificano anche il cleanup.
- La preview Flashcards richiede un lifecycle distinto dalla generazione:
  chiusura e cambio tab devono fermare interazioni e playback senza perdere gli
  edit. Il worker dedicato verifica tastiera e risposte tardive sui componenti.
- App poteva distruggersi mentre `scaleFactor` o `onResized` erano ancora pendenti,
  lasciando una registrazione nativa senza cleanup. Il worker navigazione separa
  questa responsabilità e verifica la connessione con il composition root.

Due worker `gpt-6-luna` con contesto nuovo gestiscono rispettivamente App e preview.
Il coordinatore corregge Align, revisiona implementazione/test e aggiorna i documenti.

## Evidenza finale

Il coordinatore ha revisionato i diff, richiesto e completato regressioni aggiuntive
su errori tardivi, sessioni sostituite, input IME e listener nativi; ha verificato
che gli eventi già consumati non cambino scheda. Il worker preview ha raggiunto
un limite di esecuzione prima del report finale: revisione, ultime correzioni e
gate sono stati completati dal coordinatore.

Sul checkout finale Linux:

- Gate desktop/smoke passato: **319 test frontend in 60 file**, Svelte senza
  errori/warning, build produzione, CSS a freddo, localizzazione e tre export
  APKG validi da 10 note/20 media. Dieci segnalazioni euristiche non bloccanti:
  sei note nel prodotto e quattro nei nuovi fixture dei test montati.
- Test montati su App reale con tab fixture: gli switch mantengono istanze e
  stato locale, aggiornano `active` e rilasciano registrazioni tardive al teardown.
  Il contatore della fixture dimostra continuità di stato, non un job nativo reale.
- Preview reale con store: cambio di `active` conserva edit e invalida il load
  tardivo; undo sui controlli, input nativo e player collegato; smontaggio durante
  load e vecchia sessione che non può chiudere la sostituta.
- Test Align reale: una sola apertura per evento, sospensione quando nascosta,
  ripresa alla riattivazione e cleanup alla distruzione.

- `cargo test --workspace`: **318 test passati**, compresa `whisper-bench`;
  Clippy workspace con warning come errori, formatting e whitespace passati.
  Cinque test degli hook Git e link locali dei documenti verificati.
- Otto regressioni montate della preview rieseguite dopo aver tolto dal fixture
  un cleanup che avrebbe potuto nascondere un errore nel componente reale.

Il README [stato sviluppo](../README.md) riassume tutte le aree e le priorità.
La richiesta di pubblicazione riguarda il ramo `dev`: nessuna release viene
certificata da queste sole verifiche locali.

## Cosa resta

Questi test usano happy-dom e trasporto nativo simulato: non verificano WebKit,
focus del window manager, decoder, servizi cloud o installer. Il prossimo ciclo
può estendere i test montati a stop/riprova del flusso Flashcards e ai dialoghi
provider. Recovery TSV dopo crash e migrazione keychain richiedono cambiamenti
dedicati con contratti e prove specifici.
