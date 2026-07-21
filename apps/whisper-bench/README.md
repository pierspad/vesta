# whisper-bench

Mini-app Tauri standalone per il benchmark della trascrizione Whisper di Vesta.

**Un solo eseguibile per OS.** Scegli un file audio/video (o premi
**Use bundled sample episode** per usarne uno già pronto), premi
**Run benchmark**: l'app rileva l'hardware disponibile e prova in automatico
tutte le varianti che ha senso provare su questa macchina.

| Sempre incluso nel launcher | Scaricato al volo se rilevato |
|---|---|
| `cpu`, `cpu+vad` | — |
| `vulkan`, `vulkan+vad` (se un loader Vulkan è presente) | — |
| — | `cuda`, `cuda+vad` (GPU NVIDIA rilevata) |
| — | `rocm`, `rocm+vad` (GPU AMD rilevata, solo Linux) |
| — | `sycl`, `sycl+vad` (GPU Intel rilevata) |

Il launcher pesa poco perché include solo CPU + Vulkan (un binario copre già
AMD/NVIDIA/Intel senza toolchain pesanti). CUDA/ROCm/SYCL non entrano mai nel
binario del launcher: sono `cli/srt-transcribe-cli` — la stessa identica
pipeline di trascrizione usata da Vesta e dal CLI, `lib/srt-transcribe` — con
`--features cuda|rocm|sycl`, buildati separatamente e scaricati (con verifica
sha256, una volta sola, cache locale) dalla stessa release solo se
l'hardware corrispondente viene rilevato. Il rilevamento è best-effort
(`nvidia-smi`, `lspci`/CIM, presenza del loader Vulkan): un falso positivo
produce solo una riga `error` nei risultati, non corrompe nulla.

Ogni variante produce una riga con tempo di parete, realtime factor, numero
di sottotitoli e lingua rilevata; alla fine **Download CSV** salva la
tabella. Modello Whisper, modello VAD (Silero) e — se manca — una build
statica di ffmpeg vengono scaricati automaticamente al primo avvio.

La barra di stato in alto mostra sempre cosa sta succedendo (rilevamento
hardware, download asset/worker, variante in corso con percentuale) — non
dovrebbe mai sembrare bloccata o vuota. Il log sottostante resta per il
dettaglio riga per riga; in caso di errore la variante viene marcata `error`
con il messaggio completo e il benchmark prosegue con le varianti successive
(utile per capire se una GPU non è utilizzabile).

## Build locale

```bash
# launcher (quello che gira normalmente)
cd apps/whisper-bench/src-tauri
npx @tauri-apps/cli@^2 build -- --features vulkan

# worker headless per un backend pesante, per test locali
cargo build --release --features cuda -p srt-transcribe-cli
```

## Build CI

Workflow `whisper-bench` (Actions → whisper-bench → Run workflow):

- `launcher`: builda sempre linux + windows, CPU + Vulkan, non sperimentale.
- `worker`: builda `srt-transcribe-cli` con `--features cuda|rocm|sycl`,
  opt-in dai checkbox del run manuale (toolchain pesanti, build lente),
  `continue-on-error` — nessuno di questi backend viene testato su GPU reale
  in CI (i runner GitHub non ne hanno), solo compilato.
- `zip-sample`: pacchetta la clip già presente in `Test_Subs/fixtures/detour`
  come sample scaricabile dall'app.

Tutto finisce sulla stessa prerelease rolling `whisper-bench-latest`: è da lì
che il launcher costruisce a runtime gli URL dei worker e del sample.
