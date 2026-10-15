# Vesta — stato dello sviluppo e lavoro ancora aperto

Aggiornato il **15 ottobre 2026**, sul ramo `dev`.

Questo è il punto d'ingresso per capire dove siamo e scegliere il prossimo
intervento. Riassume lo stato del codice e delle verifiche locali; non è una
certificazione della release installata. I dettagli tecnici restano in
[ARCHITECTURE](ARCHITECTURE.md), i comandi e la matrice di release in
[QUALITY](QUALITY.md), l'uso dei singoli motori nelle [guide modulo](modules/README.md).

## Valutazione attuale

La base è consolidata: tre crate fondamentali, otto librerie di funzionalità,
sette CLI e un desktop Tauri/Svelte che usa gli stessi motori. Le correzioni
recenti proteggono file già esportati, cancellazione, impostazioni e risultati
parziali. Non serve una riscrittura dell'architettura.

Il debito maggiore è nel frontend: diverse tab combinano ancora troppe
responsabilità. La deduplicazione visiva è già iniziata; il passo utile ora è
estrarre flussi completi con ownership e test, e verificare i componenti montati.
Un'altra area aperta è la validazione del prodotto installato sulle piattaforme.
Una suite locale verde non prova il comportamento di installer, Anki, decoder,
GPU e provider cloud autenticati.

Legenda: **consolidato localmente** significa che esistono implementazione e
regressioni automatiche per i casi indicati; **da estendere** significa che
restano flussi o contratti da coprire; **da validare in integrazione** richiede
ambiente installato, hardware o servizio reale. Gli stati possono coesistere.

## Stato per area

| Area | Dove siamo | Cosa resta |
|---|---|---|
| Architettura Rust | Confini `core → lib → cli/apps`; motori riusabili senza Tauri. Build CPU headless distinta dal desktop. | Conservare i confini; creare helper comuni solo per contratti davvero identici. Nessuna riscrittura generale pianificata. |
| Componenti Svelte | Picker, footer, tier, toggle e pannelli media già condivisi. Controller e workflow estratti da alcune tab. | Alleggerire Flashcards, Translate, Sync, Settings, Transcribe e Refine per responsabilità, evitando componenti con decine di props. |
| Navigazione e lifecycle | Tab mantenute montate per conservare stato e lavoro. Correzioni a shortcut di tab nascoste, eventi consumati e listener nativi tardivi. Test montati su App, Align e preview. | Estendere la copertura alle altre tab e ai dialoghi provider; verificare focus e tastiera nel webview reale. |
| Flashcards singole e serie | Preparazione configurazione, discovery limitata, run e serie separati. Snapshot delle preferenze, stop prima della partenza, risultati tardivi conservati. | Test montati stop/riprova e serie con errori intermedi; ridurre ulteriormente il coordinamento nella tab senza perdere risultati. |
| Preview ed editing | Store dedicato, filtri/paginazione/undo, invalidazione delle risposte obsolete; chiusura al cambio tab e rilascio del player. | Focus iniziale/ripristino, navigazione tastiera e accessibilità nel desktop; casi di media reali e preview molto grandi. |
| Export TSV/media | Nuovi media in staging, lock OS, rollback degli errori ordinari e preservazione del risultato precedente. | Recovery automatica dopo crash fra swap media e sostituzione TSV: non è ancora una transazione resistente ai crash. Vedere la [recovery manuale](../cli/srt-flashcards-cli/README.md#regenerating-tsv-exports). |
| APKG e import Anki | Pubblicazione atomica del file, merge degli export freschi, verifiche ZIP/SQLite/media; errore di import distinto dal successo dell'export. | Importazione e riproduzione in Anki reale, associazioni file, collisioni e serie grandi su pacchetti installati. Il merger non gestisce collezioni Anki già schedulate. |
| Traduzione | Motore condiviso, batching/failover, checkpoint, cancellazione e diagnostica provider sanitizzata; CLI con regressioni. | Provider autenticati: quota, rate limit, resume e qualità su fixture note. Proseguire l'estrazione del coordinamento dalla tab. |
| Refine/annotazione | Engine e CLI condividono annotazione tiered; pubblicazione completa di default, parziale esplicita, test HTTP locali e SIGINT. | Dialoghi montati, qualità delle annotazioni, input di dimensioni realistiche e provider autenticati. |
| Trascrizione | CPU e backend GPU alternativi; motore locale/cloud, preparazione audio e cancellazione testati; CLI JSON con record terminale. | Smoke installati CPU/GPU, timing e qualità con parlato reale. GPT-4o usa intervalli di chunk, non timing preciso per riga; la cancellazione locale non elimina job AssemblyAI remoti. |
| Sync e autosync | Retiming manuale con validazione anchor/range; autosync condiviso, cancellazione e pubblicazione protette. | Verificare allineamento contro offset noti con Whisper reale; test delle interazioni manuali e input lunghi. |
| Align | Editor affiancato e undo; shortcut ora limitati alla tab attiva, senza doppia gestione del bubbling. | Accessibilità e focus dell'editor, navigazione pagine e salvataggi/interruzioni con flussi montati più completi. |
| Estrazione sottotitoli | FFprobe/FFmpeg condivisi, salvataggi atomici, rifiuto degli alias dell'input; test subprocess. | Pacchetti installati, formati/lingue/tracce diversi; PGS/VobSub richiedono OCR, non sono convertiti come sottotitoli testuali. |
| Media e processi | Pool FFmpeg limitati, audio/preparazione con cleanup, stderr limitato, snapshot batch misurati e fallback. | Film lunghi, HEVC/VFR, spazio insufficiente, memoria e differenze fra filesystem/piattaforme. Misurare prima di ottimizzare. |
| Playback desktop | Server loopback con byte-range; preview Linux attraverso WebKit/GStreamer, separata dalle pipeline FFmpeg. | Decoder reali, seek, errori audio e dipendenze dei pacchetti su Linux/Windows. happy-dom non prova il playback. |
| Modelli e download | Download cancellabili con pubblicazione protetta, discovery invalidata quando obsoleta, risorse Whisper separate dal rendering. | Download interrotti su pacchetti installati, rimozione modelli, GPU mancante e fallback sulla macchina dell'utente. |
| Setup e configurazione | Hydration prima degli store, coda di persistenza, scrittura atomica Rust; setup attende salvataggio durevole. Config invalida segnalata. | Setup/riavvio da installazione nuova e aggiornamento; disco pieno/permessi e recupero comprensibile all'utente. |
| Credenziali e sicurezza | Chiavi nel file di configurazione; errori provider e support log riducono l'esposizione dei dati sensibili. | Keychain di sistema con migrazione compatibile e fallback dichiarato; revisione dedicata dei confini e delle informazioni diagnostiche. Il file attuale non è un keychain cifrato. |
| CLI | Sette frontend sui motori comuni, tutti con regressioni subprocess. Flashcards distingue errori, uso invalido e cancellazione. | Estendere casi di composizione fra comandi, formati e preservazione su tutte le piattaforme; documentare stdout/stderr/exit code per ogni nuovo comando. JSON solo dove esiste un contratto utile. |
| Localizzazione | Quindici cataloghi, gate su chiavi, placeholder e copie inglesi non autorizzate; controlli anche su indice Git/outgoing commit. | Revisione umana delle traduzioni e layout RTL; classificare le sei segnalazioni euristiche note su nomi/formati. |
| Accessibilità e UX | Label localizzate e primi test di eventi, notifiche, focus/hover e dialoghi. | Audit completo di focus, tab order, screen reader, zoom, contrasto e modalità compatta nel webview. La copertura attuale è parziale. |
| Errori e diagnostica | Risultati preservati, notifiche accodate e support recorder; categorie provider e retry osservabili. | Uniformare progressivamente errori strutturati agli altri confini nativi; verificare che log esportabili restino utili e privi di segreti. |
| Test e qualità | Gate unico desktop/smoke, unit/integration Rust, subprocess offline, workflow e primi componenti montati Svelte. | Più flussi montati e matrice installati; copertura esplicita dei fallimenti fra fasi e dei casi multipiattaforma. |
| Prestazioni e benchmark | Snapshot batch con confronto di output equivalente e suite documentate; smoke sintetico controlla correttezza degli export. | Input realistici, build release e stessa macchina/configurazione prima di fare nuove affermazioni di velocità. Nessun limite temporale rigido su runner rumorosi. |
| CI, hook e release | Versioni interne, i18n, formatting, Clippy e test; release vincolata al CI del commit corretto; controlli headless distinti. | Controllare il CI remoto dopo ogni push e prima di pubblicare. Non equiparare push riuscito, build e release verificata. |
| Packaging e piattaforme | Configurazioni e workflow di distribuzione presenti; dipendenze media documentate. Evidenza GPU storica nel review di release. | Eseguire e registrare la matrice Linux/Windows; macOS se distribuito. Setup, aggiornamento, playback, file association, Anki, GPU e permessi. |
| Documentazione | Architettura, qualità, guide modulo, benchmark e audit datati disponibili. Questo README riassume lo stato corrente. | Aggiornare questa pagina insieme ai cambiamenti; mantenere gli audit come prove storiche, senza duplicare specifiche discordanti. |

## Prossimi interventi, nell'ordine consigliato

1. **Flussi montati di stop/riprova e dialoghi provider.** Provare il percorso
   completo, cambio endpoint, chiusura/riapertura, fallimento del salvataggio e
   completamento tardivo; conservare risultati e impostazioni del run.
2. **Ulteriori estrazioni dalle tab.** Partire da responsabilità con lifecycle
   proprio e regressione osservabile; riusare `TierList`, `FooterActions`,
   `PathPickerField` e i workflow esistenti. Non misurare il risultato soltanto
   con le righe rimosse dal file.
3. **Recovery TSV dopo crash.** Definire journal/ripristino oppure export
   versionati con un solo riferimento finale; introdurre prove che terminano il
   processo fra le rename e verificano il riavvio su filesystem reale.
4. **Matrice dei pacchetti installati.** Registrare sistema, build, fixture,
   risultato e log per ogni voce della [matrice release](QUALITY.md#release-smoke-matrix).
5. **Keychain e migrazione.** Progettare prima il recupero e la compatibilità;
   verificare servizio assente, migrazione interrotta ed eliminazione delle
   vecchie copie senza perdere accesso alle configurazioni.
6. **Contratti nativi e copertura CLI.** Estendere errori strutturati e prove
   multipiattaforma nei punti dove aiutano davvero GUI e script.

## Come considerare chiuso un intervento

- Problema e comportamento atteso sono descritti con un caso concreto.
- Implementazione e test proteggono risultati osservabili, compresi errori e
  cancellazione quando rilevanti; un mock IPC non certifica il processo nativo.
- Check pertinenti e gate richiesti passano sul codice finale; eventuali limiti
  sono scritti, non nascosti dietro il numero dei test.
- Documenti di contratto e questa pagina sono aggiornati. Commit separati per
  cambiamenti coerenti, senza includere dati personali, chiavi o fixture enormi.
- Il CI remoto del commit pubblicato viene controllato; installer e integrazioni
  reali restano verifiche esplicite.

## Verifiche e riferimenti

Il gate ripetibile è `python3 build-scripts/quality_check.py --desktop --smoke`;
prima del push si applicano anche formatting, Clippy e test workspace indicati
in [QUALITY](QUALITY.md). I conteggi del ciclo corrente sono registrati nella
[revisione navigazione/lifecycle](reviews/navigation-lifecycle-2026-10-15.md).

- [Architettura corrente](ARCHITECTURE.md) e [qualità/release](QUALITY.md).
- [Guide dei moduli](modules/README.md) e [contribuzione](../CONTRIBUTING.md).
- [Ultimo ciclo: navigazione e lifecycle](reviews/navigation-lifecycle-2026-10-15.md).
- [Valutazione architetturale e CLI](reviews/architecture-follow-up-2026-10-15.md).
- [Cicli precedenti di affidabilità](reviews/project-review-2026-10-15.md).
- [Evidenza di release e GPU reale](reviews/release-readiness-2026-10-10.md).
- [Benchmark nativo](BENCHMARK_NATIVE.md) e [metodo di misura](BENCHMARK_STEPS.md).

Quando si chiude una priorità, aggiornare la riga dell'area, il prossimo passo e
il link alla prova. I numeri degli audit precedenti restano riferiti al rispettivo
checkpoint e non rappresentano automaticamente lo stato corrente.
