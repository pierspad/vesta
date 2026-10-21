# Revisione impostazioni, endpoint e STT

Data: 21 ottobre 2026.

## Modifiche implementate

### Menu tagliati

La causa era `contain: layout paint style` sulle card dentro `.settings-scroll`.
`paint` impone il clipping indipendentemente dallo z-index del menu. La precedente
prova isolata del pannello non riproduceva questo contenitore: per questo lo
z-index da solo non risolveva il caso reale.

Ora, mentre una card contiene il focus, vengono disabilitati il contenimento
paint e il rendering differito di quella card e viene alzato il suo livello.
Le altre card conservano le ottimizzazioni di scorrimento. La verifica visiva
è stata ripetuta usando `.settings-scroll[data-dense="true"]`: il menu supera
il bordo della card e resta sopra il pannello successivo.

### Endpoint locali modificabili

Le configurazioni salvate includono ora due voci di base:

| Voce | URL iniziale | Uso |
|---|---|---|
| Local LLM | `http://localhost:11434/v1` | Chat OpenAI-compatible |
| Local STT server | `http://localhost:8000/v1` | Trascrizione OpenAI-compatible |

Sono configurazioni, non servizi installati o avviati da Vesta. Le loro identità
restano stabili e non possono essere eliminate o trasformate in un altro provider;
URL e credenziali sono modificabili. Le configurazioni salvate prevalgono sui
valori iniziali, senza duplicare gli ID. Cambiare il server LLM aggiorna anche
l'impostazione locale usata dai percorsi precedenti dell'app.

L'URL compare nei tiers in sola lettura. La modifica avviene nella lista dei provider salvati. Nel form va inserito l'URL
**base**, normalmente comprensivo di `/v1`, non il percorso `/models` o
`/chat/completions`. Il salvataggio accetta HTTP(S) e rifiuta credenziali nell'URL,
query e frammenti: una chiave, se necessaria, va nel campo dedicato.

### Scelta dei modelli

Rimosso il catalogo statico dei modelli LLM locali, compreso il default Llama 3.3.
I nuovi endpoint locali partono senza modello selezionato. La select legge
`/models` dal server configurato; resta l'inserimento manuale del modello.
Le scelte già salvate non vengono cancellate. A server locale non raggiungibile,
il modello precedentemente salvato non viene presentato come modello scoperto.

La cache di discovery LLM include ora anche l'URL: cambiare server non riutilizza
la lista del precedente endpoint. Anche gli endpoint STT personalizzati possono
interrogare `/models`, con fallback manuale in caso di errore. Una lista generica
può contenere anche modelli non STT: bisogna scegliere un modello che il server
supporti per la trascrizione.

Un server raggiungibile che risponde 404/405 a `/models` può essere provato con
un modello manuale. Questo non certifica che l'inferenza funzionerà; la richiesta
reale restituisce l'eventuale errore. Errori di autenticazione e connessione
continuano a essere segnalati.

### Credenziali condivise e compatibilità esplicita

Le credenziali rimangono in un archivio unico. OpenAI e Groq sono riutilizzabili
nei domini LLM e STT. I cataloghi delle due sezioni mostrano solo le configurazioni
pertinenti. Un provider custom dichiara `LLM`, `STT` oppure entrambi nel form.
Cambiare la dichiarazione invalida l'uso di un eventuale tier rimasto collegato
al dominio non più supportato.

Questa dichiarazione non converte un'API: un server dichiarato LLM deve esporre
chat completions; uno STT deve esporre la trascrizione audio. Una singola chiave
può autorizzare entrambi, ma i modelli e i protocolli restano distinti.

### STT al posto di Whisper

La voce delle impostazioni si chiama ora **STT** (speech-to-text). Gli ID interni
e le chiavi di configurazione `whisper` restano invariati per compatibilità.
Whisper mantiene il proprio nome nella libreria dei modelli locali perché quel
motore integrato è realmente Whisper. Il tier predefinito continua a usare il
motore locale; il server STT locale è un'opzione separata e non lo sostituisce.

Corretto il percorso dei custom STT: è selezionabile nei tiers, la disponibilità
non richiede una chiave fittizia e il backend accetta un endpoint custom esplicito
senza autenticazione. In quel caso non invia un header Authorization vuoto.
I provider cloud nominati continuano a richiedere le proprie credenziali.

### Icone provider

Sono stati inseriti asset locali provenienti dai siti ufficiali per Google AI,
Groq, OpenAI, Mistral, Deepgram e AssemblyAI. Il componente condiviso li riutilizza
in form, card e selettori senza richieste esterne durante il rendering. Il vecchio
riquadro verde di OpenAI è stato sostituito con l'asset Blossom delle linee guida
ufficiali, presentato su fondo neutro.

Origine e data di recupero sono registrate in
`apps/srt-gui/src/lib/config/providerBrandAssets.json`. Restano fallback testuali per i marchi
senza un asset acquisito e verificato, fra cui OpenRouter e NVIDIA. Non vengono
presentati glifi inventati come loghi ufficiali aggiornati. La verifica delle
fonti è puntuale, non un sistema di aggiornamento automatico dei marchi.

Fonti di riferimento:
- [OpenAI — linee guida ufficiali](https://openai.com/brand/)
- [Mistral — brand](https://mistral.ai/brand/)
- [OpenRouter — asset ufficiali](https://openrouter.ai/brand)

## Contratto STT e limiti

Un server personalizzato funziona se implementa l'API compatibile:

- `POST {base}/audio/transcriptions` con multipart `file`, `model`,
  `response_format=verbose_json` e richiesta dei timestamp per segmento.
- Per la traduzione audio in inglese serve anche `/audio/translations`.
- Risposta JSON con segmenti `start`, `end`, `text`; è supportato anche testo
  senza segmenti, usando la durata del chunk per il timing. Quest'ultimo caso
  è meno preciso di veri timestamp del modello.

Non è sufficiente che un server esegua un modello chiamato “speech”: non sono
supportate automaticamente API WebSocket/realtime, protocolli proprietari o
server che rifiutano i parametri multipart richiesti. L'API LLM OpenAI-compatible
non implica il supporto di `/audio/transcriptions`.

## Verifiche e lavori ulteriori

Verificati: suite frontend, controllo Svelte, compilazione del backend desktop,
unit test Rust STT e una richiesta HTTP di trascrizione senza autenticazione verso
un server di prova. Il test controlla percorso, risposta e assenza di Authorization.
Sono coperti URL salvati, assenza di duplicazioni degli endpoint, capacità condivise,
custom senza chiave, discovery non implementata e tiers con capacità obsolete.

Restano da provare con server reali le combinazioni dei prodotti dell'utente
(Ollama/LM Studio per chat, faster-whisper o altri server per STT). Non sono state
usate chiavi cloud reali o chiamate a pagamento. Per supportare un motore con API
diversa occorre un adapter dedicato, con test di contratto: non basta aggiungerne
il nome alla lista dei provider. Per un server che accetta solo `response_format=json`
servirà rendere configurabile il formato o aggiungere un adapter; non viene
promesso supporto universale a tutti i modelli STT.

Esito finale della validazione: **378 test frontend superati in 74 file**, **36 test
Rust STT superati**, controllo Svelte con **0 errori e 0 avvisi**, build frontend e
`cargo check --locked -p vesta` riusciti. Un endpoint STT senza modello selezionato
non viene considerato pronto. `git diff --check` non segnala errori.


## Rifinitura provider successiva

“Saved API keys” è stato rinominato in “Saved providers”. L'azione Edit e il suo
evento sono stati rimossi dai tiers LLM e STT: gli URL restano visibili, ma si
modificano soltanto dalla lista dei provider salvati. Il selettore Usage riusa
SearchableSelect e apre verso l'alto per restare visibile dentro il form scorrevole.

I nomi devono essere unici fra tutti i provider, anche in modifica, ignorando
maiuscole, spazi equivalenti e varianti Unicode compatibili. Il controllo compare
nel form e viene ripetuto nel percorso di salvataggio. Il provider corrente è
escluso dal confronto, quindi può essere salvato mantenendo il proprio nome.

I loghi di NVIDIA, Mistral, OpenAI, OpenRouter e Gemini sono stati sostituiti con
i cinque asset indicati dall'utente. OpenRouter ha sfondo nero. File e fonti sono
nel catalogo src/lib/config/providerBrandAssets.json; le immagini restano locali.
Questa selezione sostituisce le indicazioni sui fallback riportate sopra.


## Nitidezza, menu e notifiche

I cinque marchi sono ora SVG vettoriali equivalenti alle immagini indicate:
NVIDIA, OpenRouter, Mistral e Gemini da lobe-icons, OpenAI dalla sorgente SVG
dell'immagine Wikimedia. Il catalogo conserva anche rasterSource come riferimento.
Il disegno a pixel di Mistral resta intenzionalmente squadrato; non è un difetto
di antialiasing da eliminare.

Usage apre verso il basso con icone monocromatiche. Il menu viene portato fuori
dal contenitore scorrevole del form e ancorato al campo; segue scroll e resize,
con altezza limitata allo spazio nella finestra. Test coprono selezione, chiusura
esterna e rimozione del menu allo smontaggio.

La lista Saved providers usa ora altezza naturale e padding di 12 px, con solo
un limite massimo per lo scroll: eliminati i valori fissi che stringevano la
prima riga contro il bordo inferiore.

Nel percorso Notifications non è stato trovato un comando di ricaricamento o
navigazione automatica. Il dialogo ora conserva lo scroll degli antenati del
controllo di apertura e ripristina il focus con preventScroll. Un test simula
il salto di scroll del dialogo nativo e verifica il ripristino; il test Sidebar
verifica anche che l'apertura della inbox non cambi tab.
