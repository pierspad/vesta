# Consolidamento verso una release stabile

Il prossimo ciclo riguarda correzioni, coerenza e verifiche dei percorsi esistenti.
Non aggiungere strumenti, opzioni o dipendenze per estendere il prodotto.

## Correzioni del passaggio attuale

- Snackbar: 1700 ms per info/success; 3500 ms per warning/error, salvo override.
  Rimossi i default diversi delle tab. Un nuovo messaggio sostituisce il timer
  precedente; la snackbar espone uno stato o un alert ai lettori di schermo.
- Selettori audio del setup e fallback APKG/TSV: cliccare l'opzione attiva non
  seleziona l'altra. Il pulsante ciclico dei formati flashcard resta ciclico.
- Setup: il ritorno alla sezione Whisper sopravvive al reload finale e il
  completamento attende la persistenza anche della navigazione richiesta.
- Pulsanti condivisi: hover applicato solo quando abilitati.
- README: npm e requisito Node allineati al progetto; trascrizione descritta
  come failover sequenziale frontend, distinto dal pool di traduzione Rust.
- Architettura: esplicitate le richieste HTTP frontend, l'eccezione della
  preparazione playback senza token di cancellazione, setup/prova, log e update.

Le modifiche audio e alla prova del setup del passaggio precedente sono ancora
nel checkout locale; una release già pubblicata non contiene queste correzioni.

## Verifiche automatizzate

Usare il comando documentato in [QUALITY](../QUALITY.md):

```bash
python3 build-scripts/quality_check.py --desktop --smoke
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

Il gate comprende frontend, traduzioni, CSS in sviluppo, build, test Rust e
smoke della generazione APKG. I test delle snackbar verificano la durata, la
sostituzione dei messaggi e gli override; quelli playback verificano cache e
fallback dopo una copia audio fallita.

## Prima di chiamarla stabile

1. Eseguire la matrice di installazione/setup/aggiornamento su Linux e Windows
   con pacchetti installati, non soltanto `tauri dev`. La guida pratica è
   [provare setup e installer](provare-setup-e-installer-2026-10-07.md).
2. Provare sul media che mostrava il caricamento bloccato: prima preparazione,
   riapertura dalla cache, seek e riproduzione. I test con FFmpeg simulato non
   verificano i decoder WebKitGTK/GStreamer della macchina dell'utente.
3. Importare in Anki un APKG singolo e una serie: audio, immagini, video se
   abilitato, tipo di nota `_Vesta`, annotazioni manuali/generate e salvataggio.
4. Verificare stop/riprova, rete assente, chiave non valida e risposta parziale
   con gli endpoint realmente configurati. I test locali non certificano le API
   esterne né la qualità della traduzione.
5. Verificare le finestre strette e tutti i footer con lingua italiana e inglese;
   correggere sovrapposizioni prima di modificare ulteriormente il layout.

Correggere i difetti riproducibili emersi da queste prove, poi pubblicare una
prerelease candidata. Evitare refactor estesi delle tab prima della stabile:
spostare molto codice aumenta l'area da verificare senza migliorare subito i
percorsi dell'utente. I documenti di audit datati sono registri storici, non una
garanzia sulla release corrente.
