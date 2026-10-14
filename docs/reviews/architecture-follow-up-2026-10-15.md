# Vesta: revisione architetturale dopo il consolidamento

Checkpoint iniziale su `dev`: `8f5235e` (checkout già pulito). Questa revisione
prosegue la [valutazione precedente](project-review-2026-10-15.md), che conserva
le prove dei tre cicli già conclusi. Le verifiche di questo intervento sono
registrate separatamente in fondo al documento.

## A che punto siamo

Vesta ha una struttura adatta a evolvere senza riscrivere i motori: tre crate
`core`, otto librerie e sette CLI; il desktop compone questi servizi. Sono già
presenti persistenza atomica dei singoli file, staging/rollback TSV, cancellazione,
workflow frontend testabili, discovery protetta dalle risposte obsolete e test
di componenti montati. Questi progressi non certificano la release installata.

Il problema principale è la quantità di responsabilità rimasta nelle tab.
Al checkpoint Flashcards conta 2.714 righe, Translate 1.513, Sync 1.479,
Transcribe 1.371, Settings 1.345 e Refine 1.195. I numeri aiutano a trovare i
punti da leggere: non sono un criterio sufficiente per dividere un componente.

`App.svelte` importa le tab progressivamente e le mantiene montate quando le
nasconde. Il preriscaldamento importa e monta anche tab non visitate. Quindi
`onDestroy` non equivale a cambio scheda: polling, shortcut e dialoghi devono
considerare `active`, mentre un'elaborazione in corso può continuare in background.
Questo contratto merita test espliciti prima di cambiare la navigazione.

## Deduplicazione e tab Svelte

Esistono già `TierList`, `FooterActions`, `PathPickerField`, `ToggleRow`, pannelli
media e moduli `workflows/`. Una nuova astrazione deve riusare questi contratti,
oppure spiegare una differenza reale. La deduplicazione utile separa:

- presentazione condivisa, con props e snippet limitati;
- orchestrazione specifica di un flusso, con input catturati prima degli await;
- motore Rust, che mantiene elaborazione, processi e pubblicazione degli output.

Prima estrarre una responsabilità completa e testabile da Flashcards; poi
applicare lo stesso metodo a provider/dialoghi di Translate e Refine. Non creare
un controller universale per tutte le operazioni: lock, validità dei risultati e
cancellazione hanno semantiche diverse. Non spostare un intero blocco in un file
nuovo se continua a dipendere da decine di variabili della tab.

## CLI

Sei CLI hanno già test subprocess; Flashcards è l'eccezione al checkpoint.
La priorità è verificare il binario effettivo: output dei comandi di ispezione,
errori di argomenti/input, file precedenti preservati e Ctrl+C. Il relativo
handler cancella il token, ma `main -> Result<()>` non distingue ancora
cancellazione da errore ordinario. Un contratto coerente rende affidabili gli
script senza introdurre obbligatoriamente JSON in ogni comando.

Ogni CLI resta un adattatore del proprio motore. Un crate comune è giustificato
solo quando esistono contratti identici ripetuti; il semplice uso di clap o di
un token non basta. La pubblicazione sincrona già iniziata va completata: riportare
cancellazione dopo un export riuscito sarebbe fuorviante.

## Piano e criteri di completamento

| Ordine | Intervento | Prova richiesta |
|---|---|---|
| Questo ciclo | Estrarre un flusso Flashcards e verificare il lifecycle | Input stabili, risposte superate ignorate, errori osservabili; wiring della tab controllato. |
| Questo ciclo | Rafforzare Flashcards CLI | Test subprocess offline; exit code distinto per cancellazione prima della pubblicazione; output precedente conservato. |
| Successivo | Navigazione e dialoghi montati | Cambio tab, tastiera, chiusura/riapertura, cleanup e stop/riprova sui componenti effettivi. |
| Successivo | Recovery TSV dopo crash | Processo terminato fra le rename, riavvio e ripristino deterministico; fixture su filesystem reale. |
| Dedicato | Credenziali in keychain | Migrazione compatibile, indisponibilità del servizio e recupero senza perdita delle impostazioni. |
| Prima della release | Pacchetti installati e integrazioni | Matrice Linux/Windows, Anki, GPU/fallback e provider autenticati della guida qualità. |

Implementazione e test di questo ciclo sono assegnati a due worker `gpt-6-luna`
con contesto nuovo e ownership disgiunte. Il coordinatore mantiene analisi,
documentazione, revisione dei diff e gate finale.

## Documentazione

[`ARCHITECTURE.md`](../ARCHITECTURE.md) descrive i contratti correnti;
[`QUALITY.md`](../QUALITY.md) mantiene comandi e limiti delle verifiche;
README e guide modulo spiegano l'utilizzo. Gli audit datati conservano il contesto
storico. Evitare di accumulare nuovi cicli nel medesimo audit: un follow-up breve
con link all'evidenza precedente rende più semplice distinguere stato e storia.

## Verifica finale

### Modifiche integrate

`FlashcardPreviewController` ora possiede guardie, costruzione della configurazione,
log/errori e disposal della richiesta di preview. La tab lo compone e lo disattiva
alla distruzione; `PreviewStore` mantiene dati del dialogo, IPC e invalidazione
delle risposte obsolete. È un'estrazione circoscritta: non risolve tutte le
responsabilità accumulate nelle 2.700 righe di Flashcards.

La revisione ha individuato un errore di bubbling: Escape dal campo ricerca
veniva fermato dal pannello interno e non raggiungeva il comando di chiusura.
Ora chiude anche da quel campo. I test montati usano il vero store, verificano
chiusura/riapertura con un vecchio errore IPC tardivo e il listener undo prima
e dopo lo smontaggio. I test del controller coprono disposal con callback ed
errore tardivi. Il wiring `onDestroy` della tab è stato letto direttamente;
la tab intera non è montata da queste nuove regressioni.

La CLI Flashcards distingue ora il risultato non riuscito con token cancellato
(exit 130) dal successo pubblicato (exit 0). Conserva l'output testuale e respinge
qualità snapshot fuori da 0–100. Cinque test subprocess coprono riepiloghi,
argomenti/input invalidi, successo senza media, fallimento FFmpeg e SIGINT con
handshake e timeout. Le due prove di fallimento/cancellazione verificano i byte
del TSV e dei media precedenti. Il contratto è nella
[guida CLI](../../cli/srt-flashcards-cli/README.md#script-contract).

### Evidenza del coordinatore

Sul checkout Linux finale:

- `python3 build-scripts/quality_check.py --desktop --smoke`: passato; 299 test
  frontend in 56 file, Svelte senza errori/warning, build e CSS a freddo passati,
  localizzazione senza errori bloccanti e tre APKG da 10 note/20 media verificati.
  Restano sei segnalazioni euristiche di localizzazione già note.
- `cargo test --workspace`: 318 test passati, compresa `whisper-bench`.
- `cargo clippy --workspace --all-targets -- -D warnings`, formatting e
  whitespace: passati. Cinque test degli hook Git passati.
- Otto regressioni preview rieseguite dopo il rafforzamento finale del test undo.

Il coordinatore ha revisionato sorgenti e test, richiesto casi aggiuntivi ai due
worker e verificato autonomamente il gate. Le prove montate usano happy-dom e IPC
simulato; quelle SIGINT sono Unix. Installer, playback WebKit/GStreamer,
importazione Anki, GPU e provider autenticati non sono stati verificati in questo
ciclo. Non è stato eseguito push né pubblicata una release.
