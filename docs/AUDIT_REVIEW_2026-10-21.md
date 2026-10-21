# Verifica del report di audit — 21 ottobre 2026

Il report sul Desktop è stato trattato come un insieme di ipotesi da verificare.
Il checkout conteneva già numerose modifiche non committate: le correzioni
preesistenti sono state conservate, senza attribuirle a questa revisione.

## Decisioni

- **Mantenere i crate riutilizzabili separati.** Anteprima, allineamento,
  sottotitoli e trascrizioni rappresentano contratti diversi. Contare undici
  strutture non dimostra undici duplicazioni. Cast numerici e piccoli adapter
  non giustificano da soli una migrazione incompatibile del parser pubblico.
- **Mantenere srt-apkg.** Il reimpacchettamento di archivi esistenti e la
  creazione streaming con manifest hanno responsabilità diverse. Non spostare
  SQLite e i modelli Anki soltanto per uniformare il nome del crate.
- **Mantenere CLI autonome e router vesta-cli.** I runtime Tokio dei comandi
  non vengono tutti avviati contemporaneamente: viene eseguito il comando
  selezionato. Tre righe di main non sono un costo significativo da eliminare.
- **Mantenere vesta-mcp opzionale.** È un processo stdio separato avviato dal
  client, senza porta di rete e senza servizio permanente installato. Offre
  schemi degli strumenti, risultati strutturati, ID dei job, progressi e
  cancellazione. Usa lo stesso motore delle flashcard; non riduce i costi LLM
  né accelera il motore. Non copre ancora tutte le operazioni della CLI.
- **Conservare le API Rust pubbliche deprecate compatibili.** L'assenza di
  chiamanti nel repository non dimostra l'assenza di utenti esterni.
- **Non rimuovere benchmark, fixture o test di migrazione per la loro età.**
  Servono ancora se verificano formati o percorsi supportati. Nomi di vecchi
  modelli nei dati fittizi non rendono obsoleto un test di serializzazione.

## Correzioni già presenti e verificate

Il checkout contiene la validazione dei timestamp VTT (con controllo overflow),
la pubblicazione APKG/TSV su spawn_blocking, la risoluzione FFmpeg in un modulo
media comune, la protezione dalle risposte audio obsolete, l'uso di ankiStore
per l'importazione automatica e la rimozione di componenti orfani, chunk
CodeMirror, cache Vite tracciata e lockfile pnpm disallineato.

I checkpoint della traduzione ora scrivono fuori dai worker asincroni, ma
mantengono un lock e un salvataggio per batch. È una correzione del blocco di
Tokio, **non** un checkpointing meno frequente. Conserva il recupero dopo ogni
batch; l'eventuale riduzione delle scritture richiede una decisione esplicita
sulla perdita accettabile di lavoro e misure prestazionali.

## Dizionario effettivamente collegato alle carte

Nessun HSK/CEFR. L'opzione **Impostazioni → Anki → Dizionario nelle carte APKG**
aggiunge link alle parole del testo originale nei template esportati. È
facoltativa, attiva per impostazione predefinita (rispettando una disattivazione
esplicita già salvata) e non modifica
il testo memorizzato nelle note.

La lingua proviene dal normale target_language: tipo di nota scelto per film,
poi inferenza dal nome del sottotitolo e preferenza di studio secondo le regole
esistenti. Il riconoscimento del testo contribuisce alla scelta automatica del
tipo di nota nella GUI. La lingua si corregge scegliendo il tipo di nota per
film; non si deduce dalla lingua dell'interfaccia.

Intl.Segmenter separa le parole secondo la lingua della carta, anche senza
spazi. La segmentazione non equivale a lemmatizzazione o disambiguazione:
forme flesse, nomi propri e composti possono richiedere una selezione manuale.
Il link apre il Wiktionary inglese (dizionario multilingue), nella sezione della
lingua quando nota e presente; non sceglie automaticamente un dizionario bilingue.
Le definizioni non vengono scaricate. Nessuna richiesta parte prima del clic.

Nei webview senza Intl.Segmenter, o senza lingua nota, resta la ricerca tramite
selezione del testo. Il lookup opera sui blocchi `.expression`, presenti nei
template standard; i template personalizzati devono conservarli. Vale per APKG:
un TSV non può installare template nel modello Anki di destinazione. Non cambia
le carte già esportate. Verificato con test DOM e template Rust; l'apertura del
browser su ogni versione di Anki desktop/mobile richiede una prova reale.

## Test e codice mantenuto soltanto dai test

Rimossi dopo verifica dei chiamanti:

- Vecchio runFlashcardGeneration, relativi adapter e quattro test del percorso
  non più usato. Conservato il tipo della risposta IPC; aggiunta la verifica di
  un solo film attraverso runFlashcardSeries, effettivamente usato dalla GUI.
- Vecchio copyText e due test duplicati: restano writeClipboard/readClipboard
  e i test di successo, rifiuto e clipboard vuota.
- generateDefaultDeckName e due test: resta deriveDeckNameFromFile, che è il
  percorso effettivo e conserva titoli con numeri.
- matchResolutionPreset e formatBytes con tre test di funzioni senza chiamanti.
  Restano i test dei controlli di qualità usati dall'interfaccia.
- getLanguageOptions: le verifiche utili ora coprono localizedLanguageName e
  getLanguageSearchTerms, usati dall'app.
- filterSubtitleTracks: i test esercitano preparazione e filtro effettivi;
  la sola composizione di comodo è stata spostata nel test.
- isTrustedSupportUrl, usato solo dal test: il test verifica direttamente gli
  URL del catalogo, senza spedire un validatore inutilizzato nell'app.

I test del vecchio catalogo/store dizionari erano già rimossi con il sottosistema
scollegato. Il nuovo lookup verifica segmentazione giapponese, conservazione di
ruby e link, esecuzioni ripetute, fallback manuale, opt-in e sicurezza della lingua
inserita nei template. I test MCP usano un vero client stdio e producono esportazioni,
quindi verificano un'integrazione reale e non giustificano codice fittizio.

## Limiti residui

Non è una certificazione di assenza di debito tecnico. Restano possibili lavori
mirati: polling dell'anteprima tradotta che rilegge i file, errori di traduzione
classificati tramite stringhe e API desktop deprecate da migrare con verifica
su piattaforme reali. L'effect dei filtri scrive nello stato osservato ma converge
entro i limiti: dal codice non emerge il ciclo infinito affermato dal report.
Non sono stati introdotti grandi accorpamenti o velocizzazioni non misurate.

## Validazione eseguita

- `npm test -- --reporter=dot`: 72 file, 370 test superati.
- `npm run check`: 0 errori, 0 avvisi Svelte.
- `npm run build`: completata.
- `cargo test --locked -p srt-parser -p srt-apkg -p srt-flashcards -p srt-translate -p vesta-mcp -p vesta-cli`: superato.
- Test aggiuntivo Rust dell'inserimento opt-in nei due template: superato.
- `cargo check --locked -p vesta` e `cargo check --locked -p vesta-cli`: completati.
- `git diff --check`: nessun errore di whitespace.

Non eseguito un test manuale in un'installazione reale di Anki. Non eseguita
l'intera matrice hardware e piattaforme del workspace.

## Rifinitura successiva delle impostazioni

Il dizionario è ora attivo di default anche nei payload senza preferenza esplicita;
un opt-out salvato viene rispettato. La lingua dei preset personalizzati viene
salvata e caricata dal normale percorso dei note type, quindi guida font e lookup
anche quando il nome del file suggerisce una lingua diversa.

Il selettore mostra bandiere e gruppi ordinati (personalizzati prima dei
predefiniti). Il campo lingua è sempre visibile. Il salvataggio verifica nome
non vuoto, nomi già occupati e schemi identici per campi e lingua, sia nel pulsante
sia nello store. Le tre opzioni Anki hanno un layout comune con descrizioni più
brevi in italiano e inglese. Ridotto il margine fra chiavi API e tiers da 40 a
16 px nelle due sezioni. ApiKeysCard e TierList erano già condivisi: non è stato
aggiunto un ulteriore livello generico agli editor specifici.

Validazione: 373 test frontend superati in 73 file; controllo Svelte senza errori
o avvisi; build frontend riuscita; suite Rust flashcard/MCP superate. Controllo
visivo del pannello isolato con IPC fittizio: bandiere, lingua del preset,
validazione e ordine dei gruppi verificati. Nessuna configurazione reale modificata
dalla prova del pannello. Il controllo i18n non segnala errori; restano avvisi
preesistenti su testi tecnici e fixture.
