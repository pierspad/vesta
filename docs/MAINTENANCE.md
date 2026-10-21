# Manutenzione

Questo documento raccoglie il debito ancora aperto. Le procedure riproducibili
sono in [QUALITY](QUALITY.md); architettura e contratti pubblici sono descritti
in [ARCHITECTURE](ARCHITECTURE.md) e [API_COMPATIBILITY](API_COMPATIBILITY.md).
Le precedenti analisi datate sono recuperabili dalla cronologia Git.

## Interventi consolidati

- Anteprima traduzione: cache di una coppia di file, con limite stimato di
  32 MiB e invalidazione tramite dimensione/mtime e identità del file su Unix.
  Quando cambia l’output viene riparsato il file tradotto; l’input è riutilizzato.
  Modifiche che conservano deliberatamente tutti i metadati non sono rilevate.
- Lettura, parsing e preparazione dei job di traduzione su worker dedicati.
  La cancellazione durante la preparazione attende il worker e impedisce
  successive richieste ai provider, conservando la guard dell’operazione.
- Template HTML/CSS Anki condivisi fra Rust e GUI; script e personalizzazioni
  restano separati. Il default Google usa un’unica costante nei costruttori e nel pool.
- Controller audio comune per trasporto, riproduzione, invalidazione e cleanup.
- Chunk cloud letti con I/O asincrono cancellabile, senza aumentare la concorrenza.
  Interrompere l’attesa non garantisce di interrompere I/O già avviato dal runtime.
- Costruttori HTTP fallibili nei percorsi che possono propagare l’errore;
  wrapper pubblici compatibili con avviso esplicito sul fallback legacy.

## Lavoro aperto

| Area | Prossimo passo e vincolo |
| --- | --- |
| Checkpoint traduzione | Misurare byte scritti, attesa del lock e throughput prima di introdurre writer/journal. Preservare ordine e durabilità per batch; aggiungere prove crash/failure prima di cambiare protocollo. |
| Pipeline cloud | Misurare latenza, RAM e quote prima di sovrapporre preparazione/invio o aumentare concorrenza. Definire ordinamento progressi e comportamento al primo errore. |
| Provider HTTP | Valutare helper circoscritti per prompt/retry conservando autenticazione, payload, trimming e classificazione degli errori. |
| API desktop deprecate | Migrare apertura file/cartelle e policy WebKit con verifica nel desktop reale. |
| Responsabilità GUI | Estrarre flussi completi dalle tab quando esiste un confine concreto, conservando ownership di cancellazione e cleanup. |
| CI | Misurare setup cold/warm prima di condividere artefatti o introdurre dipendenze fra job. Conservare la verifica CPU headless indipendente. |
| Lookup ID SyncEngine | Aggiungere un indice solo se la frequenza e le dimensioni giustificano memoria e obblighi di aggiornamento. |
| Errori legacy | Conservare il fallback testuale per adapter custom; migrare verso errori tipizzati con un piano di compatibilità. |
| Dipendenze frontend | Analizzare le 7 vulnerabilità segnalate da `npm ci` sul lockfile verificato (2 moderate, 5 high); non applicare aggiornamenti automatici senza valutare impatto e regressioni. |
| Prodotto installato | Completare smoke con Anki, provider autenticati, decoder, installer e hardware reale secondo QUALITY. |

## Verifica delle correzioni

Il passaggio di manutenzione ha superato 390 test frontend (76 file), 380 test
Rust workspace, Svelte/TypeScript senza errori o avvisi, Clippy workspace con
warning trattati come errori, formattazione, build frontend e test CSS. Gli hook
pre-commit/pre-push sono stati eseguiti normalmente e la CI della PR è passata.
Questi risultati descrivono quel passaggio; per lo stato del commit corrente
usare i comandi di QUALITY e la relativa CI.

La regressione della cache verifica 1.000/10.000/50.000 sottotitoli: dieci
richieste identiche non aumentano i due parsing iniziali e restituiscono dati
identici e ordinati. Non impone soglie temporali e non misura la traduzione
end-to-end:

```sh
cargo test --locked -p vesta commands::translate::context_tests::preview_cache_debug_timings_for_1k_10k_and_50k_subtitles --lib -- --nocapture
```

I test audio coprono richieste obsolete, chiusura durante generazione, errori e
completamenti/rifiuti tardivi di `play()`. Non sostituiscono una prova dei codec
nel webview. I test cloud usano file e HTTP loopback, senza inferenza a pagamento.
