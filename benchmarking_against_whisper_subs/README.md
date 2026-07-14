# Benchmark — Vesta (whisper-common) vs [`whisper-subs`](https://lib.rs/crates/whisper-subs)

Confronta la pipeline di trascrizione di Vesta (crate interno `whisper-common`,
la stessa usata da GUI e CLI) con il crate esterno
[`whisper-subs`](https://github.com/ldicocco/whisper-subs) (whisper-rs +
Silero VAD, beam search, chunked decoding).

```bash
./benchmarking_against_whisper_subs/1_setup.sh          # build + modelli (una volta)
./benchmarking_against_whisper_subs/2_run_benchmark.sh  # esegue e scrive i report
```

Output in `results/` (gitignorata):

| File | Contenuto |
|---|---|
| `results.csv` | una riga per variante: score, WER, timing, wall, CPU, RSS, MiB |
| `summary.md` | stessa tabella in markdown, ordinata per score |
| `timings.csv` | misure grezze di `/usr/bin/time -v` per ogni run |
| `raw/*.srt` | i sottotitoli prodotti da ogni variante |
| `raw/*.time.*` | log grezzi di time |

## Cosa viene misurato

- **Accuratezza** — WER del testo trascritto contro `Test_Subs/FILM/Detour-en.srt`
  (409 cue umane, OpenSubtitles). Non è una trascrizione verbatim, quindi il
  WER assoluto è un proxy; il **confronto relativo** tra le varianti è il dato
  affidabile (stessa reference per tutti).
- **Timing** — hit-rate delle cue di riferimento coperte ≥50% + MAE degli
  start dei segmenti.
- **Score 0–100** — 70% testo, 30% timing (formula in `report/score.py`).
- **Velocità/risorse** — wall clock, CPU user+sys, %CPU, picco RSS.
- **Peso nella build** — dimensione dei due binari release e del modello
  (colonne `binary_mib` / `model_mib`). Nota: entrambi linkano whisper.cpp
  staticamente, quindi il binario di `whisper-subs` è una buona stima dei MiB
  che il crate aggiungerebbe se adottato dentro Vesta.

## Le varianti

| Variante | Decoder | VAD | Note |
|---|---|---|---|
| `vesta` | greedy (best_of 1), threads = cores−1 | no | pipeline attuale di Vesta |
| `whisper-subs-fast` | greedy, `--logprob-threshold -1.0` | no | stesso decoder di Vesta: confronto ad armi pari |
| `whisper-subs-best` | beam 5, logprob −0.5 | Silero v5.1.2 | i default "quality-first" del crate |

Tutte e tre usano **lo stesso file modello** (`~/.cache/whisper/ggml-small.bin`,
override con `WHISPER_BENCH_MODEL=medium`), build **plain CPU** per entrambi i
tool — niente CUDA/BLAS/Metal, così si misura il codice, non il backend.

`whisper-subs` è installato pinnato (v0.1.1, `--locked`) in `.bin/` per
riproducibilità. `REPEATS=3 ./2_run_benchmark.sh` per mediane su più run.
