# Provare setup, audio e installer

## Setup senza cambiare le preferenze

Aprire **Sperimentale → Prova senza salvare**. Si apre lo stesso componente del primo avvio: percorso rapido oppure personalizzato, lingue, esportazione, audio e trascrizione. Chiudere in qualsiasi momento oppure terminare il percorso. Questa modalità non salva preferenze, non cambia la lingua dell'app e non scarica font, Whisper o VAD. Non simula il download: serve a verificare i passaggi e le scelte dell'interfaccia.

Non esiste ancora un tutorial separato dei singoli strumenti: il percorso ripetibile è il setup iniziale.

## Setup reale

**Sperimentale → Ricomincia setup** riavvia l'app e forza il percorso reale. Completandolo si sostituiscono le preferenze configurabili nel setup; chiavi API e altre preferenze restano nel relativo archivio.

Il percorso rapido imposta le lingue, l'interfaccia semplice, APKG e MP3. Il percorso personalizzato permette di scegliere formato, fallback, audio MP3/Opus e trascrizione locale. Se richiesta, scarica il modello Whisper selezionato e Silero VAD; riusa quelli già presenti. Entrambi scaricano i font necessari alle lingue scelte. Le preferenze vengono salvate al termine; se un download fallisce il setup mostra l'errore e permette di riprovare. I download completati non vengono rimossi.

Per verificare il primo avvio autentico, usare un account di sistema o una VM nuova. Installare Vesta, aprirlo, completare il setup e riaprire l'app: il percorso deve apparire soltanto al primo avvio. Non occorre cancellare la configurazione usata normalmente.

## Audio della sincronizzazione

1. Caricare un SRT e un MP3: deve essere riproducibile senza conversione.
2. Caricare un MKV con Opus/Vorbis: la preparazione prova prima a copiare la traccia audio senza ricodificarla.
3. Provare un MKV con audio incompatibile con OGG: passa alla conversione audio, ottimizzata per l'ascolto dei dialoghi. L'esportazione delle flashcard mantiene le proprie impostazioni audio.
4. Ricaricare lo stesso file: viene riutilizzata la cache completa.
5. Provare un file non valido: deve apparire un errore e il controllo per scegliere un altro file. Ogni processo FFmpeg ha un limite di cinque minuti.
6. Senza media, “Conferma e avanti” resta disabilitato e non reagisce al passaggio del mouse.

La durata della prima preparazione dipende dalla durata e dal codec della traccia. La velocità sul file dello screenshot non è stata misurata: il file sorgente non è disponibile nella richiesta.

## Installer e aggiornamenti

Il setup iniziale configura Vesta dopo l'installazione; l'installer installa invece il programma sul sistema. Sono due percorsi distinti.

I pacchetti della release `v0.26.0-dev.1` sono stati compilati con successo per Linux, Windows, Flatpak e Arch. Provare Windows e Linux in VM dedicate: installazione, avvio, setup, chiusura, riapertura e disinstallazione. La compilazione automatica non verifica le finestre dell'installer su una macchina reale.

L'aggiornamento interno consulta l'ultima release **stabile**, sceglie l'installer per sistema e architettura, scarica il file, verifica SHA-256 e apre l'installer. Supporta Windows, DEB e RPM; AUR, Flatpak, Snap e AppImage richiedono il relativo gestore o la pagina delle release. Una build avviata con `tauri dev` non usa il percorso di installazione automatica.

Per provare davvero l'aggiornamento interno, installare in VM una versione stabile precedente: **Impostazioni → Panoramica → Controlla aggiornamenti**, poi usare l'azione di installazione quando disponibile. Verificare che non si avvii l'installer se il download o il checksum falliscono. Le prerelease di `dev` vanno installate dalla pagina della release: il controllo dell'ultima stabile non le propone.
