# Riesame dei flussi e della modalità semplice di Vesta

Data: 7 ottobre 2026. Base: `dev` aggiornato ad `a00cf93`, con il lavoro locale preservato. Questo documento distingue interventi già realizzati, problemi osservati e proposte ancora da implementare.

## Giudizio

La modalità semplice è una buona riduzione dei controlli tecnici, ma non è ancora un percorso sufficientemente guidato per una persona nuova. È più semplice soprattutto per chi conosce già media, sottotitoli e Anki. Restano troppe decisioni simultanee, terminologia interna, impostazioni di funzioni facoltative presentate come problemi e passaggi manuali fra strumenti.

La scelta di prodotto che consiglio è questa: l'utente decide **che cosa vuole ottenere, quali contenuti usare e in quale lingua**. Vesta decide i parametri di esecuzione. Le scelte con conseguenze su contenuto, spesa, servizi esterni e sovrascrittura rimangono visibili e deliberate.

Non cambierei indiscriminatamente i valori salvati per rendere la schermata apparentemente semplice. Distinguerei le preferenze avanzate conservate dai parametri effettivi della modalità semplice. Tornando in modalità esperto, la persona deve ritrovare le proprie impostazioni.

## Metodo e limiti

Ho esaminato shell e navigazione, setup iniziale, tutte le tab operative, impostazioni, componenti comuni, store di configurazione, matching dei file, esportazione, gestione delle lingue e comandi Rust interessati dalla traduzione. Ho verificato visivamente gli stati vuoti di traduzione e sincronizzazione e il caricamento di un campione originale nella traduzione, con un backend Tauri simulato e dati fittizi.

La simulazione serve a verificare rendering, disposizione, controlli e transizioni senza leggere preferenze reali, inviare contenuti o chiamare un modello. Non sostituisce una prova completa del pacchetto desktop con video, dialoghi nativi, Anki e provider AI. I test automatici coprono la logica separatamente. Non ho condotto uno studio con utenti: le valutazioni di difficoltà sono conclusioni progettuali motivate dal codice e dalla UI, non risultati di usabilità misurati.

## Interventi realizzati in questo passaggio

1. **Label condivisa degli stati vuoti.** `EmptyStatusLabel.svelte` mantiene testo, stile e annuncio accessibile coerenti. Usata in estrazione, allineamento, lista sottotitoli della sincronizzazione, area di riproduzione della sincronizzazione e anteprima di traduzione. I segnaposto sono statici: non simulano un caricamento in corso.
2. **Traduzione già disposta prima del caricamento.** Originale e tradotto sono sempre presenti. Su schermi stretti le colonne si impilano. Il contenuto di ogni colonna ha altezza limitata e scorrimento, evitando di allungare indefinitamente la pagina.
3. **Anteprima reale dell'originale.** Il comando che carica l'SRT restituisce anche le prime dieci righe normalizzate, con i loro ID. Nessuna richiesta AI aggiuntiva. Durante la traduzione si torna a mostrare le coppie originali/tradotte più recenti.
4. **Due decisioni tecniche rimosse dalla modalità semplice.** Blocchi di traduzione da 15 sottotitoli e sovrapposizione di 2; sono gli stessi valori iniziali già usati. I controlli numerici rimangono in modalità esperto e le preferenze avanzate della sessione non vengono sovrascritte quando si usa quella semplice. Questa è una configurazione fissa prudente, non un adattamento automatico al modello.
5. **Contesto facoltativo progressivo.** Nella modalità semplice il pannello è inizialmente chiuso e si apre quando serve. Il contesto ricavato dai metadati continua a essere incluso automaticamente; il suggerimento AI rimane un'azione esplicita perché richiede una chiamata aggiuntiva.
6. **Lingua selezionabile prima dei file.** Traduzione e trascrizione non obbligano più a caricare un file prima di impostare la lingua.
7. **Motivo di blocco leggibile nella traduzione.** La barra inferiore mostra il prerequisito mancante; quando manca la configurazione AI offre un pulsante diretto alle impostazioni.
8. **Percorso di output automatico distinto da quello manuale.** Cambiando sorgente o lingua, Vesta aggiorna il nome automatico. Una destinazione scelta esplicitamente viene preservata. Quando la lingua del nome originale coincide con quella di destinazione, il nome automatico aggiunge `translated` per evitare di restituire il percorso sorgente. Vale anche per percorsi Windows.
9. **Stato coerente al cambio di file.** Le informazioni e l'anteprima della sorgente precedente vengono azzerate. Le risposte asincrone relative a percorsi precedenti non sostituiscono quelle attuali. Avvio e riprova richiedono un SRT caricato con almeno un sottotitolo.
10. **Controlli di traduzione bloccati durante l'esecuzione.** File, lingua e contesto non possono cambiare durante l'elaborazione, evitando che i controlli mostrino una configurazione diversa da quella effettivamente in esecuzione.

## Scelte predefinite consigliate

| Decisione | Default consigliato | Motivo e possibilità di correzione | Stato |
| --- | --- | --- | --- |
| Lingua madre | Quella scelta nel setup | È una preferenza personale, non va indovinata ad ogni operazione | Già presente |
| Lingua di studio | Quella scelta nel setup | Dà una base a matching, font e tipo di nota | Già presente |
| Lingua del singolo file | Metadati/tag attendibili, poi lingua di studio | Mostrare quale lingua è stata riconosciuta; rendere semplice correggerla | Presente in parte |
| Tipo di nota | Automatico per lingua, con suffisso `_Vesta` | Mantiene schemi coerenti; i tipi personalizzati rimangono espliciti | Già presente |
| Formato flashcard iniziale | APKG | Un file importabile e comprensibile; preservare il pulsante ciclico richiesto | Già presente, con incoerenze fra modalità da correggere |
| Importazione diretta in Anki | Solo quando l'utente ha scelto AnkiConnect | Non dipendere silenziosamente da Anki aperto o da un plugin installato | Conservare scelta esplicita |
| Audio | Attivo, traccia della lingua studiata | Chiedere solo quando il riconoscimento è ambiguo; offrire ascolto di controllo | Matching già presente |
| Codifica audio semplice | MP3, 128 kbit/s | Compatibilità iniziale; Opus resta una scelta consapevole di compattezza | Default già presente |
| Normalizzazione audio | Attiva | Riduce salti di volume; evitare di annullare una scelta avanzata esplicita | Default presente; effetto di forzatura da correggere |
| Immagini | Attive se è disponibile un video | Supporto visivo con poco costo; qualità iniziale da verificare su schermi reali | Default presente |
| Clip video | Disattive | Risparmiano tempo e dimensione; attivabili dall'utente | Già presente |
| Dimensione e qualità dei media | Un solo profilo consigliato | Nella schermata iniziale non occorrono tre preset per ogni tipo di media | Proposto |
| Nome del mazzo | Titolo ricavato dai file | Mostrare il risultato prima della generazione, consentire modifica | Già presente in parte |
| Serie/film | Ricavato dai file importati | Evitare una decisione preliminare quando basta contare e raggruppare i file | Proposto; matching già disponibile |
| APKG separati per episodi | Conservare il default attuale, scelta avanzata | Modificarlo automaticamente altererebbe l'organizzazione dei mazzi esistenti | Non cambiato |
| CPU | Budget automatico che lasci margine al sistema | Non chiedere numero di core nella UI semplice; valutare il default per carico e memoria | Parzialmente presente |
| Output | Destinazione e nome generati, visibili e modificabili | Togliere la scelta obbligatoria senza nascondere dove finisce il risultato | Migliorato per traduzione |
| Blocchi/overlap traduzione | 15 / 2 | Riduce due decisioni tecniche, senza nuove chiamate | Realizzato |
| Provider/modello AI | Quello configurato dalla persona | Non scegliere autonomamente servizi a pagamento o cloud; failover nei limiti configurati | Conservare scelta esplicita |
| Contesto AI | Metadati locali automatici, annotazioni opzionali | Non inventare trama o titolo; la proposta AI rimane su richiesta | Già presente; disclosure realizzata |
| Trascrizione locale | Modello installato e compatibile | Suggerire un modello adatto al dispositivo; rendere download e dimensione espliciti | Proposto |
| VAD | Se disponibile e scelto, usarlo; altrimenti spiegare il comportamento | Evitare una schermata che sembra non funzionare perché manca un accessorio | Parzialmente presente |
| Annotazioni AI | Solo carte senza annotazioni, blocchi attivi | Protegge lavoro già svolto ed evita richieste inutili | Default già presente |
| Salvataggio | Nuovo file come azione primaria quando si modifica un originale | Non confondere comodità con sovrascrittura automatica | Proposto |

## Riesame per area

### Navigazione e setup

**N1 — Prima scelta già tecnica.** `FirstRunSetupModal.svelte` inizia chiedendo setup rapido oppure personalizzato. La persona deve scegliere la complessità prima di vedere il prodotto. Consiglio un solo setup iniziale con lingua madre e lingua di studio; “altre impostazioni” deve essere secondario. Il setup avanzato resta accessibile dalle impostazioni.

**N2 — Scelta UI/lingua madre.** La lingua madre imposta anche la lingua dell'interfaccia quando supportata, altrimenti inglese. È un default ragionevole, ma il riepilogo dovrebbe mostrare entrambe: chi studia una lingua non necessariamente vuole l'interfaccia in quella lingua.

**N3 — Download nel completamento del setup.** Il completamento può scaricare font e, nel setup personalizzato, Whisper/VAD. Proposta: riepilogo degli elementi da scaricare con avanzamento per elemento e recupero dopo un errore. Il setup di base non dovrebbe fallire in modo poco comprensibile perché un font non è raggiungibile.

**N4 — Troppi strumenti allo stesso livello.** Flashcard, estrazione, trascrizione, traduzione, sincronizzazione, revisione, annotazione e sperimentale sono tutte voci di navigazione. Proposta: mantenere una vista “strumenti”, ma offrire anche un flusso iniziale “Crea flashcard” che suggerisca solo il passaggio necessario. Non obbligare l'utente a conoscere già la pipeline.

**N5 — Sperimentale.** La tab contiene attualmente il riavvio del setup. Questa funzione appartiene alle impostazioni; una tab distinta aumenta la navigazione senza rappresentare un'attività quotidiana.

**N6 — Sidebar collassata.** Le icone con tooltip possono funzionare per utenti esperti; per la prima sessione le etichette sono più utili. Preservare la preferenza esplicita; evitare che un cambio di dimensione cancelli la scelta dell'utente.

Riferimenti: `src/App.svelte`, `components/Sidebar.svelte`, `modals/FirstRunSetupModal.svelte`, `tabs/ExperimentalTab.svelte`.

### Flashcard

**F1 — Funzioni multimediali troppo ampie nella modalità semplice.** Audio, immagini e video occupano pannelli distinti con scelte e preset. Proposta: primo livello con tre interruttori e un riepilogo “audio e immagini inclusi; video escluso”, dettagli apribili nel pannello o per episodio. Mantenere l'anteprima facilmente raggiungibile.

**F2 — Preset semplici ma ripetuti.** “Leggero/bilanciato/alto” ripetuto per più tipi di media non elimina la decisione, la moltiplica. Proposta: un profilo consigliato e un accesso ai dettagli. Non cambiare subito qualità e risoluzione per utenti esistenti: il default attuale delle immagini è il preset leggero e va valutato su carte reali.

**F3 — Accoppiamento inatteso.** Il preset delle immagini modifica anche il bitrate quando l'audio usa Opus (`SnapshotsPanel.applyQualityStep`). Una scelta visiva dovrebbe dichiarare le conseguenze sull'audio, oppure agire solo sulle immagini. È un buon candidato per un vero profilo multimediale unico.

**F4 — Audio normalizzato forzatamente.** `AudioClipsPanel` contiene un effetto che riporta `normalizeAudio` a `true`. Il default è sensato, ma questo annulla anche una preferenza esplicita. Proposta: usarlo come default iniziale, oppure come valore effettivo della modalità semplice, senza riscrivere la preferenza avanzata.

**F5 — Traccia audio.** Il riconoscimento automatico è già un punto forte. Migliorerei il riepilogo con “Giapponese, traccia 2” e aprirei la selezione solo se ci sono più candidati credibili. Non scegliere semplicemente la prima traccia quando potrebbe essere un doppiaggio o un commento.

**F6 — Caricamento e matching.** Il pulsante “Carica media” e il matching di sottotitoli/media evitano molti clic. Mancano ancora una distinzione immediata fra associazioni certe e dubbie e un percorso breve di correzione. Le associazioni ambigue vanno evidenziate prima di generare centinaia di carte.

**F7 — Tipo di nota nel footer.** L'automatismo per lingua è buono; la lista completa di lingue/tipi può diventare una decisione superflua. Consiglio di mostrare inizialmente il tipo rilevato come riepilogo con azione “cambia”, preservando il suffisso `_Vesta` e i tipi personalizzati. La correzione manuale deve restare possibile.

**F8 — Formato e modalità semplice.** Lo store della modalità cambia il formato quando si passa da esperto a semplice, mentre il footer consente di cambiarlo anche nella modalità semplice. Serve un comportamento unico: APKG iniziale, scelta manuale persistente, nessun reset inatteso dovuto al solo cambio di modalità. Il pulsante ciclico appena ripristinato va mantenuto.

**F9 — Scelta per episodio/unico pacchetto.** È sempre visibile anche quando non applicabile. Si può mostrarla solo per una serie con più episodi e output APKG, oppure metterla nei dettagli. Il default attuale è un pacchetto per episodio; non lo cambierei senza decidere quale organizzazione dei mazzi vuole il prodotto.

**F10 — Nome e destinazione.** Nome automatico e cartella predefinita già riducono lo sforzo. Prima di avviare servirebbe un riepilogo concreto: numero di episodi, lingua, nome del mazzo, media inclusi e destinazione. Una cartella obbligatoria con asterisco sembra richiedere una scelta anche quando è già stata compilata.

**F11 — Effetti della modalità.** I commenti citano CPU `n-1` e formato forzato, ma `effectiveCpuCores` è attualmente un alias del valore salvato. Occorre verificare e centralizzare i parametri effettivi: nascondere un controllo non deve lasciare attiva una scelta esperta imprevista.

Riferimenti: `tabs/FlashcardsTab.svelte`, `panels/AudioClipsPanel.svelte`, `panels/SnapshotsPanel.svelte`, `panels/VideoClipsPanel.svelte`, `stores/uiModeStore.svelte.ts`, `stores/generationStore.svelte.ts`, `utils/mediaSettings.ts`, `workflows/flashcardSeries.ts`.

### Estrazione dei sottotitoli

**E1 — Il risultato è già raggruppato per tracce e varianti.** È una buona base. Consiglio di preselezionare lingua di studio e lingua madre e distinguere “estrai le lingue utili” da “mostra tutte le tracce”. Non nascondere completamente lingue non riconosciute.

**E2 — OCR e formati.** Per una traccia non testuale, spiegare l'azione necessaria invece di aspettare un errore di estrazione. Codec e numeri di traccia restano nei dettagli.

**E3 — Output.** La cartella è già suggerita. Il passaggio successivo dovrebbe essere un'azione “usa nelle flashcard” o “traduci questo sottotitolo”, senza riaprire il file picker.

**E4 — Stati distinti.** Nessun media caricato, scansione in corso, nessuna traccia disponibile e filtro senza risultati sono quattro situazioni diverse. Il codice già distingue la scansione completata; il riesame ha mantenuto questa distinzione insieme alla label standard.

Riferimenti: `tabs/ExtractTab.svelte`, `components/SubtitleTrackCard.svelte`, `utils/subtitleTracks.ts`.

### Traduzione

**T1 — Parametri interni nella modalità semplice.** Risolto con default 15/2. Non affermerei che un blocco piccolo sia sempre “più preciso”: la qualità dipende anche da contesto, modello e lunghezza del testo. Un futuro adattamento deve basarsi sul testo/token e sugli errori effettivi, non sul solo nome del preset.

**T2 — Originale invisibile fino all'avvio.** Risolto: un campione locale appare al caricamento. Questo consente di controllare di aver scelto il file corretto prima di usare un servizio AI.

**T3 — File di output.** Risolti i casi del nome automatico uguale all'originale e del nome non aggiornato al cambio di sorgente. La protezione non comprende alias di filesystem, hard link o un percorso manuale coincidente: serve una verifica robusta lato Rust prima di scrivere. La destinazione manuale non viene modificata automaticamente.

**T4 — Configurazione AI.** Il percorso reale usa i tier, ma la tab conserva numerosa logica del vecchio selettore provider/modello. Suggerisco di rimuovere quel percorso inattivo dopo una verifica dei riferimenti e delle migrazioni; oggi complica manutenzione e ragionamento sulla disponibilità. Il caricamento del file deve restare indipendente dalla configurazione del modello.

**T5 — Prerequisiti.** Migliorati con testo visibile e accesso alle impostazioni. Il prossimo passo è sostituire “configura tier” con un setup guidato “scegli come tradurre” e poi mostrare il motore scelto come riepilogo.

**T6 — Ripresa e riprova.** “Riprova” dopo qualsiasi risultato non distingue traduzione riuscita, interrotta e incompleta. Proposta: apri risultato quando completa; riprendi quando parziale; riprova quando fallita. Non avviare chiamate duplicate senza chiarire quali righe verranno elaborate.

**T7 — Contesto.** Il suggerimento AI ha un costo aggiuntivo e non è necessario per iniziare. Il disclosure appena introdotto riduce la pressione a compilarlo. Manteniamo annotazioni manuali e metadati affidabili; non trattiamo un titolo dedotto dal filename come una trama verificata.

Riferimenti: `tabs/TranslateTab.svelte`, `panels/TranslationPreviewPanel.svelte`, `services/translate.ts`, `utils/translationPaths.ts`, `src-tauri/src/commands/translate.rs`.

### Trascrizione

**TR1 — Lingua bloccata prima del caricamento.** Risolto. La persona può scegliere prima la lingua; il setup e l'ultima scelta restano i riferimenti.

**TR2 — Backend pronti contro backend configurati.** La tab distingue già entry utilizzabili da quelle soltanto salvate. La UI iniziale dovrebbe mostrare una sola indicazione “Trascrizione locale pronta” oppure un'azione “prepara la trascrizione”, con modello mancante/download direttamente raggiungibili.

**TR3 — Parametri avanzati nascosti ma persistenti.** Qualità e VAD sono salvati; la modalità semplice nasconde i controlli. Verificare la configurazione effettiva dopo una sessione esperta e descrivere eventuali fallback. Non scaricare modelli aggiuntivi automaticamente durante un'operazione.

**TR4 — Modello consigliato.** Se c'è un solo modello pronto, selezionarlo senza chiedere. Se ce ne sono più di uno, proporre quello configurato e mostrare nome/lingue come riepilogo. Il modello da scaricare richiede una scelta informata su dimensione e risorse del dispositivo.

**TR5 — Risultato e passaggio successivo.** L'anteprima dei segmenti è utile. A completamento, offrire “usa come sottotitoli”, “traduci” o “crea flashcard”; oggi il flusso lascia ancora passaggi di file manuali.

Riferimenti: `tabs/TranscribeTab.svelte`, `panels/TranscriptionSegmentsPanel.svelte`, `workflows/transcription.ts`, `workflows/transcriptionResources.svelte.ts`.

### Sincronizzazione

**S1 — Stati vuoti.** Realizzate label coerenti per lista sottotitoli e area media. I controlli e le righe restano predisposti; non spariscono interi pannelli quando manca un file.

**S2 — Manuale e automatico simultanei.** La UI mostra statistiche, ancore, offset, wizard, sessioni e due modalità automatiche insieme. Proposta: automatico come percorso suggerito se i prerequisiti sono pronti; controlli manuali sotto “correggi la sincronizzazione”. Conservare sempre il lavoro manuale e consentire controllo del risultato.

**S3 — Modalità automatica.** Il default `quick` è già presente. Si può eliminare la scelta iniziale rapido/accurato e offrire un'azione “migliora il risultato” se la verifica è insoddisfacente. Non avviare automaticamente un'elaborazione più lunga solo perché il punteggio è basso senza chiarire durata e comportamento.

**S4 — Pulsanti segmentati.** Entrambi i pulsanti di modalità chiamano `toggleMode`, quindi cliccare quello già selezionato cambia comunque modalità. Consiglio che ogni segmento imposti il proprio valore, come già corretto per alcune scelte nelle flashcard.

**S5 — Sessione contro risultato.** “Salva sessione” e “Salva file” sono semanticamente diversi ma visivamente simili. Azione primaria: salva sottotitoli sincronizzati. Sessione: opzione secondaria per continuare il lavoro. Una sessione automatica di recupero sarebbe utile, ma deve rispettare percorso e consenso al salvataggio.

**S6 — Confidenza.** Mostrare `0%` quando nulla è caricato può sembrare un giudizio di qualità. Usare un trattino nello stato vuoto e spiegare se la confidenza misura l'allineamento, non la correttezza complessiva del file.

Riferimenti: `tabs/SyncTab.svelte`, `panels/SubtitleListPanel.svelte`, `components/WizardCheckpoint.svelte`, `components/AutoSyncControls.svelte`, `stores/autoSyncStore.svelte.ts`.

### Revisione e annotazioni

**R1 — Allineamento/revisione a due file.** Il riconoscimento di un sottotitolo compagno è già presente. Migliorerei la terminologia per specificare chiaramente quale lato si modifica e quali timing si mantengono. Le modifiche vanno controllate prima di esportare.

**R2 — Editor avanzato accessibile anche in modalità semplice.** La revisione mantiene navigazione delle righe, salti ai vuoti e strumenti dettagliati. È appropriato per un editor, ma non dovrebbe sembrare un passaggio obbligatorio nella pipeline di ogni nuovo utente.

**R3 — Modifiche non salvate.** Esiste una protezione nel caricamento tramite picker dell'allineamento. Va verificata anche per trascinamento, cambio tab e chiusura della finestra: la protezione deve coprire ogni modo di sostituire il documento, non un solo pulsante.

**R4 — Annotazione manuale come partenza.** `RefineTab` parte in modalità manuale: buon default, nessuna chiamata AI implicita. Solo carte non annotate e uso dei blocchi sono già attivi per l'automazione. Mantenerli.

**R5 — Azione sulla singola carta versus tutto il mazzo.** Rendere evidente la quantità coinvolta e il criterio “solo senza annotazioni” prima di avviare l'AI. Il raggruppamento non deve far supporre che si tratti sempre di una sola richiesta.

**R6 — Salvataggio.** La funzione di salvataggio sullo stesso file esiste. Proposta: “Salva copia” primaria per una prima modifica, sovrascrittura esplicita secondaria; mostrare numero di carte modificate e preservare ID/schema/media. Non scegliere automaticamente di eliminare note esistenti.

**R7 — Istruzioni AI.** Le istruzioni personalizzate e i prompt sono impostazioni avanzate. Un buon prompt iniziale dovrebbe evitare una scelta obbligatoria, ma lingua, stile e risultato desiderato sono preferenze dell'utente: non sostituirle con una deduzione silenziosa.

Riferimenti: `tabs/AlignTab.svelte`, `tabs/RefineTab.svelte`, `config/refinementPrompt.ts`, `src-tauri/src/commands/refine.rs`, `lib/srt-refine`.

### Impostazioni e componenti comuni

**C1 — Funzioni facoltative presentate come setup incompleto.** La lista delle azioni richieste considera modello Whisper e tier LLM mancanti anche se l'utente vuole soltanto creare flashcard da file esistenti. Consiglio un indicatore di problema solo per la funzione che si sta cercando di usare. Nelle impostazioni, le altre sono “funzioni da attivare”, non errori.

**C2 — Configurazione dei tier troppo tecnica.** Per il caso comune, impostazione guidata con provider e modello, che costruisce internamente un singolo tier. Priorità, endpoint multipli, RPM e budget rimangono nei dettagli. Non scegliere servizi o modelli a pagamento autonomamente.

**C3 — Lingue.** `LanguageSelect` e il catalogo unico sono la base corretta. La lista dei tipi di nota è diversa da una select della sola lingua: non va forzata nel medesimo componente, ma deve usare lo stesso catalogo e le stesse normalizzazioni. I nomi `_Vesta` sono parte della compatibilità Anki, non solo una label estetica.

**C4 — Barra inferiore stretta.** Osservato nella verifica visiva a circa 960 px: nella sincronizzazione i gruppi si sovrappongono/tagliano. Il minimo della finestra desktop è 460 px, mentre `FooterActions` ha altezza fissa e gruppi che non si spezzano. Priorità alta: rendere responsive i gruppi, spostare le azioni secondarie in un menu e mantenere sempre raggiungibile l'azione primaria. Verificare anche flashcard, non correggere un'unica tab.

**C5 — Select ricercabili accessibili.** `SearchableSelect` usa un input e pulsanti per le opzioni; le label con `for` non sempre puntano a un ID esistente. Consiglio una revisione comune di ID, nome accessibile, ruolo combobox, apertura, elemento attivo e navigazione da tastiera. Il componente condiviso permette una correzione per tutta l'app.

**C6 — Pannelli vuoti e caricamento.** La label standard realizzata qui risolve una parte. Distinguere sempre assenza di input, lavoro in corso, dati vuoti validi ed errore; non usare skeleton animati a riposo. Le label non devono coprire controlli attivi né prendere il focus.

**C7 — Azioni disabilitate.** Un tooltip non basta su touch e tastiera. La traduzione ora ha testo e azione visibili; applicare gradualmente lo stesso schema a trascrizione, generazione e sincronizzazione, con un solo prossimo passo prioritario.

**C8 — Backup e diagnostica.** Backup esportabile e diagnostica sono utili ma secondari. Mantenerli disponibili senza renderli parte del normale percorso di creazione. La registrazione dei log resta una scelta esplicita e va associata al problema da risolvere.

**C9 — Persistenza.** La configurazione durevole e il flush prima del riavvio del setup sono buone basi. Qualunque semplificazione deve preservare impostazioni, template, schemi e preferenze salvate; il cambio di modalità non è un reset.

Riferimenti: `tabs/SettingsTab.svelte`, `panels/OverviewSettingsPanel.svelte`, `components/TranslationTiers.svelte`, `components/TranscribeTiers.svelte`, `components/FooterActions.svelte`, `components/SearchableSelect.svelte`, `config/vestaConfig.ts`.

## Ordine di lavoro che consiglio

1. **Barre azioni responsive e salvataggio sicuro.** Risolvono comandi non raggiungibili e rischio di modificare il file sbagliato. Aggiungere anche le protezioni lato backend per percorsi sorgente/output e recupero del lavoro non salvato.
2. **Modalità semplice coerente nelle flashcard.** Media inclusi come tre scelte, un profilo consigliato, riepilogo lingua/traccia/mazzo/output. Conservare il pulsante ciclico del formato e le correzioni manuali. Centralizzare i parametri effettivi senza cancellare preferenze esperte.
3. **Setup AI guidato e avvisi contestuali.** Eliminare la sensazione di installazione incompleta per chi non usa AI. Configurare una volta il motore, mostrare solo stato e azione necessaria nelle tab operative.
4. **Passaggi diretti tra strumenti.** Estratto → traduci → usa nelle flashcard; trascritto → traduci/crea carte. Riusare il risultato in memoria/percorso senza chiedere nuovamente quale file scegliere.
5. **Onboarding per obiettivo e revisione accessibilità.** Un percorso iniziale orientato al risultato, seguito dai dettagli quando servono. Verificare con persone nuove: quanti clic, quante domande prima del primo risultato, quante correzioni di associazioni e quanti ritorni alle impostazioni.

Non inizierei da un redesign grafico generale: i maggiori miglioramenti vengono dal togliere decisioni e rendere chiaro il prossimo passo.

## Verifica della modifica

- Controllo Svelte/TypeScript senza errori o warning.
- Test frontend, controllo traduzioni e build eseguiti.
- Test della libreria backend Vesta, incluso caricamento di un SRT da 12 righe: campione limitato a 10, ID/testi preservati, originale invariato.
- Regressioni dei nomi di destinazione: separatori, alias ISO, percorsi Windows, directory con nomi simili a lingue e lingua di output uguale a quella della sorgente.
- Verifica UI simulata: stati vuoti di sincronizzazione/traduzione, caricamento originale prima dell'AI, default semplice e disponibilità dei controlli esperti.

Restano da provare nel desktop reale dialoghi nativi, ciclo completo con provider AI e combinazioni di media/Anki. Non sono stati eseguiti commit o push.
