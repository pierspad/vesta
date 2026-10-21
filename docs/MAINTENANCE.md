# Manutenzione

Le procedure riproducibili sono in [QUALITY](QUALITY.md); architettura e contratti
pubblici in [ARCHITECTURE](ARCHITECTURE.md) e [API_COMPATIBILITY](API_COMPATIBILITY.md).
Questo documento descrive i confini da mantenere durante le modifiche.

## Frontend

I tab compongono le viste e mantengono il collegamento con le preferenze. Le
responsabilità verificabili separatamente vivono in `apps/srt-gui/src/lib/workflows`:

- Flashcards: preparazione episodi, generazione serie, anteprima e selezione
  tracce audio. Un probe vecchio non deve modificare la scelta di un altro file.
- Translate: discovery modelli per identità provider/credenziale; risultati
  obsoleti non devono aggiornare lo stato o la cache. Un errore lascia il fallback
  statico e non deve generare un ciclo di richieste automatiche.
- Transcribe: risorse Whisper e sottoscrizioni native; le registrazioni che
  terminano dopo lo smontaggio devono essere immediatamente rilasciate.
- Refine: indicizzazione/ricerca carte e protocollo dei job automatici. Il listener
  deve precedere l'invocazione, filtrare il run ID e venire sempre rilasciato.
- Settings: preferenze provider e bozza JSONC di smart matching. Le regole salvate
  e quelle ricaricate devono attraversare la stessa normalizzazione.
- Sync: gli aggiustamenti temporali accorpati per frame devono essere cancellati
  al reset e allo smontaggio.

Non spostare indiscriminatamente effetti reattivi in un controller comune:
traduzione, trascrizione e generazione hanno cancellazione e ownership differenti.
Preservare UI, eventi di preferenze e cleanup quando si estrae una responsabilità.

## Persistenza e compatibilità

I checkpoint di traduzione sono snapshot atomici serializzati sotto lock. Si
salva dopo ogni batch: liberare il lock prima della scrittura può pubblicare
snapshot fuori ordine. Prima di cambiare protocollo misurare byte cumulativi,
attesa del lock e throughput; verificare crash, cancellazione e disco pieno.

`get_missing_subtitle_ids` è un'API pubblica compatibile che controlla solo ID
assenti; la sostituta verifica anche righe scorrette. Il campo legacy
`FlashcardConfig::use_timings_from` è accettato ma la generazione usa sempre i
timing target; la GUI non offre un selettore che prometta un comportamento diverso.
Conservare migrazioni di preferenze e fallback degli adapter custom.

## Verifiche e benchmark

```sh
python3 build-scripts/quality_check.py --desktop --smoke
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo run --locked --release -p srt-translate --example checkpoint_bench -- /percorso/storage
cargo test --locked -p vesta commands::translate::context_tests::preview_cache_debug_timings_for_1k_10k_and_50k_subtitles --lib -- --nocapture
```

La suite comparativa, i generatori dei report, le fixture e le istruzioni in
`benchmarking_against_subs2srs/` e `benchmarking/` sono strumenti di riproducibilità,
anche quando non sono importati dall'applicazione. Non eliminarli come report
transitori. I risultati sintetici non misurano provider, contesa o prestazioni
end-to-end; non giustificano aumenti automatici della concorrenza.

Prima della release restano necessari gli smoke installati descritti in QUALITY:
Anki/associazioni, codec webview, persistenza, cancellazione, provider autenticati,
installer e hardware reale. Ripetere i controlli advisory dei lockfile; distinguere
vulnerabilità, manutenzione e unsoundness della catena GTK/Tauri, senza ignore.


## Dipendenze desktop transitive

Tauri 2.12 usa ancora GTK3/GLib 0.18. La correzione ufficiale di
[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429) è in GLib
0.20+, che non sostituisce direttamente i tipi richiesti da GTK3. L'avviso
riguarda `VariantStrIter`: non viene soppresso né considerato risolto soltanto
perché il codice Vesta non usa direttamente quell'iteratore. Una copia locale
con backport richiede test e manutenzione propri; il controllo advisory può
continuare a segnalarne la versione anche dopo una correzione del sorgente.

Restano inoltre gli avvisi di manutenzione per `proc-macro-error` (macro GTK) e
`zvariant`, `zvariant_derive`, `zbus_names` (catena D-Bus di dialoghi/Opener).
La loro rimozione richiede aggiornamenti dei consumatori upstream: aggiungere
una seconda versione diretta al workspace non elimina quella transitiva.
Vedere anche le [indicazioni upstream zbus](https://github.com/z-galaxy/zbus/issues/2043).
Non migrare l'applicazione a release alpha o introdurre ignore solo per ridurre
il numero di messaggi. Ripetere `cargo audit` e `cargo tree -i NOME_CRATE` quando
si aggiorna questa catena.
