# Benchmark completo del motore Rust di Vesta

L'ottimizzazione è in `lib/srt-flashcards/src/snapshot_batch.rs`, condivisa da
CLI e desktop. L'app non usa Python. `verify_native.py` è soltanto un runner di
benchmark: avvia il binario Rust e verifica i mazzi. Il prototipo Python che
coordinava FFmpeg è stato rimosso e archiviato fuori dal repository.

## Avvio quando il PC è libero

Dalla radice del repository:

```bash
./benchmarking/snapshot_batch/run-benchmark.sh --dry-run
./benchmarking/snapshot_batch/run-benchmark.sh
```

Il secondo comando compila il binario release **prima** delle misure. Sono
necessari Cargo, Python 3, FFmpeg e FFprobe. `psutil` è opzionale: se presente
si misurano anche RSS e processi; senza, tempi e verifica di qualità funzionano
comunque. Nessun download, modifica ai film o dipendenza Python nell'app.

Il runner scopre ricorsivamente i film in `Test_Subs/FILM` e cerca i sottotitoli
`NomeFilm-en.srt`, poi `NomeFilm.srt`. Gestisce anche `Detour(1945).mp4` con
`Detour-en.srt`. I film senza sottotitoli vengono elencati come esclusi.
Attualmente trova **8 film utilizzabili**; `8 e mezzo.mp4` manca dei sottotitoli.

La matrice predefinita comprende, per ogni film:

- 25, 100, 300 carte e tutte le carte del film, con tempi reali dei sottotitoli;
- 100 carte sparse sull'intero film;
- WebP 256×144 (dimensioni predefinite dell'app) e 640×360;
- tre coppie A/B alternate: originale `--no-optimize`, poi ottimizzato, invertendo
  l'ordine nella seconda ripetizione;
- audio MP3 128k/44.1kHz/stereo, traccia 0, normalizzazione attiva, qualità WebP 80,
  crop/padding zero, 15 worker richiesti (il motore li limita ai core disponibili).

Con gli input attuali sono **480 generazioni**: riservare diverse ore, senza
altri benchmark, compilazioni o carichi pesanti concorrenti. Non c'è il vecchio
limite cumulativo di 600 secondi. Ogni generazione ha un timeout di due ore,
configurabile con `--timeout` (0 lo disabilita).

## Risultati e ripresa

I risultati vengono salvati fuori dal repository, in
`../vesta-benchmark-results/AAAAMMGG-HHMMSS/`:

- `summary.md`: mediane dei tempi, riduzioni o regressioni, coppie completate,
  stato ed esclusioni;
- `results.json`: comandi, versione FFmpeg/FFprobe, hash del binario e sottotitoli,
  dimensione/data dei video, tempi, hash di ogni media e dei campi delle note;
- sottotitoli selezionati e log stdout/stderr per ogni generazione.

Ogni APKG supera integrità ZIP/SQLite, conteggi delle carte e dei media, controllo
dei riferimenti, media non vuoti e confronto esatto di MP3/WebP e campi delle
note. Una differenza o un errore ferma la suite con exit code non zero e conserva
il mazzo problematico. I mazzi verificati vengono cancellati per limitare lo
spazio; `--keep-decks` li conserva. Serve comunque spazio per un mazzo completo
alla volta, più log e risultati.

Ctrl+C o SIGTERM terminano i processi della generazione attiva e salvano lo
stato. Per riprendere, usare **gli stessi argomenti** e la directory precedente:

```bash
./benchmarking/snapshot_batch/run-benchmark.sh --resume --output /percorso/risultati
```

La ripresa richiede lo stesso runner, binario, impostazioni, input e versioni degli
strumenti; salta le generazioni già verificate. Non riprende risultati con
errori di validazione. L'intervallo fra sessioni può alterare le condizioni del
PC: per misure più omogenee preferire un'esecuzione ininterrotta.

## Varianti

Solo film interi, alle dimensioni predefinite, senza campioni aggiuntivi:

```bash
./benchmarking/snapshot_batch/run-benchmark.sh --sizes all --profiles default --sparse 0
```

Altra directory, lingua, worker e output:

```bash
./benchmarking/snapshot_batch/run-benchmark.sh --input /percorso/film --language it --jobs 8 --output /percorso/nuovi-risultati
```

`--help` elenca tutte le opzioni. Per usare un binario già compilato, chiamare
`python3 benchmarking/snapshot_batch/verify_native.py` con gli stessi argomenti.

## Cosa dimostra

Il tempo comprende l'intera generazione audio + immagini + APKG: metadati,
packet scan, estrazione, copie e packaging. Python non estrae o pre-elabora
media. La cache del sistema operativo non viene svuotata; l'ordine A/B alterna
il vantaggio di una cache già calda. RSS/processi sono campionati ogni 20 ms e
possono perdere picchi o processi brevi. Non si misurano rendering della GUI,
clip video, sincronizzazione o trascrizione. Un risultato negativo indica una
regressione; coppie incomplete non dimostrano un guadagno.

Il batch Rust è limitato a CFR H.264/HEVC, larghezza 1280–1920 e altezza ≤1080,
WebP e stream non ambiguo. Raggruppa al massimo otto richieste in otto secondi,
selezionando PTS reali e preservando il comportamento ai keyframe riordinati.
Fonti/timestamp incerti, formati non supportati e richieste isolate mantengono
l'estrazione originale. Non c'è una cache persistente. I guadagni dei casi già
misurati non provano un'accelerazione universale.

Verifica del runner, senza benchmark completo:

```bash
python3 -m unittest discover -s benchmarking/snapshot_batch -p test_runner.py
```

## Grafici dai risultati completati

Non serve ripetere le misure per disegnare i grafici. Il comando usa `.venv`
se presente, altrimenti Python di sistema; richiede Matplotlib:

```bash
./benchmarking/snapshot_batch/plot-results.sh /percorso/risultati/results.json
```

Produce SVG e PNG in `charts/` accanto ai risultati, per tutti i film interi e
ogni profilo misurato. Rifiuta suite incomplete o confronti non validati.
Questi grafici confrontano Vesta originale e ottimizzata, non subs2srs.
