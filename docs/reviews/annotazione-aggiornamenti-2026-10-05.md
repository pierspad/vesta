# Revisione annotazione e aggiornamenti — 5 ottobre 2026

## Valutazione

La separazione tra annotazione manuale e automatica è valida. Conviene mantenere lo stesso elenco di card e lo stesso modello di salvataggio per entrambe, migliorando riconoscimento dei campi, gestione delle operazioni e resoconto finale. La revisione ha individuato problemi reali di integrità dei dati oltre alle variazioni di layout.

## Annotazione: problemi e interventi

| Area | Problema riscontrato | Comportamento risultante |
| --- | --- | --- |
| Mazzi Anki generici | In assenza di un campo riconosciuto, veniva usato l'ultimo campo. Un mazzo Front/Back poteva perdere la traduzione. | È richiesto un campo Notes/Annotations identificabile, distinto da fronte e retro. Campi ambigui o metadati mancanti producono un errore prima di salvare. Gli alias vengono normalizzati con trim e minuscole. |
| TSV | Il caricamento eliminava alcune righe, ma il salvataggio assegnava ID alle righe fisiche. | ID coerenti con le righe fisiche, incluse le posizioni occupate da righe vuote e direttive Anki; anche le card con fronte vuoto rimangono accessibili. |
| TSV a due colonne | Le note risultavano vuote, ma non potevano essere salvate. | La colonna Notes viene aggiunta dopo i campi presenti, preservando la traduzione. |
| TSV con campi aggiuntivi | Una colonna testuale poteva essere scambiata per Notes. | Con più di tre colonne testuali serve una direttiva `#columns:` con nomi espliciti. Un campo Notes non finale viene riconosciuto dall'intestazione. |
| File mancante | Una cache globale condivisa poteva far usare il backup di un altro mazzo. | Il salvataggio si ferma se manca il file originale. La cache globale è stata rimossa. |
| Scrittura | Una scrittura fallita poteva lasciare un output parziale. | TSV e APKG vengono preparati in un file temporaneo nella stessa directory e sostituiti solo dopo il completamento. |
| Metadata Anki | Il salvataggio ricalcolava sort field e checksum dal fronte, anche per modelli con ordinamento differente. | Sort field e checksum restano invariati; le note modificate aggiornano `mod` e `usn=-1`. Le note identiche non vengono riscritte. Media e altri campi vengono preservati. |
| ID non validi | Alcuni ID errati, duplicati o mancanti venivano ignorati. | Errore esplicito; il file originale resta intatto. |
| Generazione manuale | La card selezionata può cambiare mentre arriva una risposta. | La risposta viene applicata all'oggetto della card originale; la card attualmente selezionata resta editabile se non è in generazione. |
| Concorrenza | Manuale e automatico non usavano lo stesso blocco backend. | Stato e token condivisi, con rilascio e annullamento automatici tramite guard. Una seconda richiesta concorrente viene respinta. |
| Annullamento | Il token veniva controllato solo dopo la risposta HTTP. | L'annullamento interrompe anche l'attesa della richiesta HTTP e del rate limiter. È disponibile anche durante la generazione manuale e con kill switch attivo. |
| Risposte batch | I duplicati potevano scegliere arbitrariamente l'ultima risposta; il prompt non portava le note precedenti. | ID duplicati, mancanti, vuoti e JSON malformato vanno in fallback singolo. Il payload include note precedenti e istruzione interpolata per card. |
| Risposta singola vuota | Veniva conteggiata come successo e poteva cancellare note esistenti. | Conteggiata come fallimento, senza applicare note vuote. |
| Progresso | Errori e card non elaborate erano poco distinguibili. | Conteggi separati per riuscite, fallite e rimaste. La percentuale misura le card elaborate, incluse quelle fallite. Il riepilogo distingue l'esito dalla percentuale. |
| Eventi | Eventi di un altro run potevano raggiungere lo stesso listener globale. | Ogni run ha un ID; vengono accettati soltanto i suoi eventi. |
| Memoria e lookup | Log senza limite e lookup lineari ripetuti per tutte le card. | Massimo 150 righe di attività, indice per ID e insieme reattivo delle card in elaborazione. |

## Interfaccia

- Elenco vuoto con sagome di altezza coerente con le card caricate e spazio di scrollbar riservato.
- Anteprime fronte/retro con altezza fissa; contenuto del mazzo mostrato come testo nella preview, senza eseguire HTML importato.
- Nome del file a destra di Sovrascrivi, con ellissi e percorso completo nel tooltip. Le azioni di esportazione conservano il loro spazio.
- Due colonne già da 768 px; a larghezze inferiori i pannelli si impilano.
- I pulsanti Manuale/Automatico selezionano esplicitamente la modalità: cliccare quella già attiva non cambia modalità.
- Selezione riallineata quando cambia la ricerca; messaggio esplicito se non ci sono risultati. La modifica delle note non sposta automaticamente l'editor.
- Caricamento e salvataggio bloccati durante generazione; editor bloccato durante salvataggio. Anche l'apertura del dialogo Salva è protetta.
- Opzioni del run bloccate mentre è attivo; azione Interrompi con stato Interruzione.
- Progresso presente anche prima dell'avvio, riepilogo persistente e dettaglio attività consultabile.

## Aggiornamenti

Il controllo usa la release stabile pubblicata su GitHub, con fallback al redirect ufficiale `/releases/latest`. Il precedente fallback a `package.json` su main è stato rimosso: può annunciare una versione prima che gli installer siano pronti. Le prerelease vengono confrontate correttamente con la release stabile della stessa versione. Se la versione installata è sconosciuta, viene mostrato un errore anziché dichiarare l'app aggiornata. Il controllo manuale rimane disponibile anche dopo errori o quando il controllo automatico è attivo.

| Installazione | Azione |
| --- | --- |
| Windows diretto | Scarica l'EXE ufficiale per l'architettura e apre l'installer. |
| Pacchetto DEB/RPM | Scarica il pacchetto corrispondente e lo apre con il gestore associato dal desktop. La disponibilità di tale gestore dipende dal sistema. |
| AUR/repository Arch | Indica helper AUR per `vesta-bin`, oppure pacman per un repository configurato. |
| Flatpak | Indica gestore Flatpak o `flatpak update com.vesta.desktop`. |
| Snap | Indica il gestore Snap. |
| AppImage, sviluppo, piattaforma non riconosciuta | Accesso alla pagina ufficiale della release; nessuna sostituzione automatica del binario. |

Il pulsante Scarica e installa compare solo se esiste un asset compatibile, con dimensione valida e digest SHA-256. Il backend risolve nuovamente la release ufficiale, verifica tag, piattaforma, architettura, origine dell'URL, limite dimensionale e checksum. File incompleti o non verificati non vengono aperti. Download concorrenti sono respinti; l'interfaccia mostra la percentuale e ripulisce il listener anche in caso di errore. L'utente completa l'installazione nel normale installer: Vesta non dichiara l'aggiornamento già installato.

Questo flusso non configura il plugin updater Tauri con un manifest firmato. SHA-256 verifica l'integrità rispetto al digest servito dall'API GitHub tramite HTTPS; non equivale alla firma con chiave privata di un updater autonomo. Per quel percorso occorrono una chiave persistente, artefatti firmati e relativo indice delle versioni, come descritto nella [documentazione ufficiale Tauri](https://v2.tauri.app/plugin/updater/). La pipeline attuale produce DEB/RPM ed EXE/MSI; non produce AppImage.

## Verifica e limiti

Verifica finale: 204 test frontend passati, tutti i test Rust del workspace passati, Clippy del workspace senza warning bloccanti, controllo Svelte con zero errori e zero warning, build e verifica CSS riuscite, audit delle 15 lingue senza problemi bloccanti.

Sono coperti da regressioni: roundtrip TSV con righe vuote, direttive e fronte vuoto; aggiunta Notes nei TSV a due colonne; intestazioni con Notes non finale; ID errati e duplicati senza alterazione del sorgente; rifiuto dei campi ambigui; roundtrip APKG con media e preservazione di sort/checksum; esportazione APKG→TSV; richiesta HTTP pendente annullata; pool esaurito; batch parziali/duplicati; prompt con note precedenti; mutua esclusione backend; versioni e prerelease; selezione installer; controlli aggiornamenti concorrenti/offline/errori; cleanup dei listener dopo un errore di installazione.

La verifica visiva nel browser usa IPC simulato e include stato vuoto, nome lungo, generazione manuale con cambio selezione e automatico con un successo e un fallimento. Non costituisce una prova sull'app nativa né una generazione con un LLM reale. Non è stato eseguito un installer reale durante la verifica.

Per i mazzi Anki con nomi di campo completamente personalizzati occorre ancora rinominare il campo dedicato o introdurre in futuro una selezione esplicita dei campi: questa revisione sceglie di fermarsi davanti all'ambiguità. I TSV supportati sono a righe separate e campi delimitati da tab; gli a capo nelle note vengono serializzati come `<br>` e i tab interni come spazi. Il formato APKG richiede `collection.anki2`. L'elenco non è virtualizzato: per decine di migliaia di card resta un miglioramento distinto da valutare con una misura di prestazioni.

## Release

Il commit `ee80636` porta su main il fix già committato su dev per l'estrazione dei sottotitoli. La release stabile risultante è `v0.25.1`. Tutte le build sono concluse con successo e il job [Publish to AUR](https://github.com/pierspad/vesta/actions/runs/37354817541) è completato con successo. Il lavoro descritto sopra rimane locale sul checkout dev, insieme alle modifiche preesistenti preservate.
