# whisper-bench

Mini-app Tauri standalone per il benchmark della trascrizione Whisper di Vesta.

Scegli un file audio/video, premi **Run benchmark** e l'app esegue la
trascrizione in tutte le varianti supportate dalla build corrente:

| Build | Varianti eseguite |
|---|---|
| cpu-only | `cpu`, `cpu+vad` |
| `--features vulkan` | `cpu`, `cpu+vad`, `vulkan`, `vulkan+vad` |
| `--features cuda` | `cpu`, `cpu+vad`, `cuda`, `cuda+vad` |
| `--features rocm` | `cpu`, `cpu+vad`, `rocm`, `rocm+vad` |

Ogni variante produce una riga con tempo di parete, realtime factor,
numero di sottotitoli e lingua rilevata; alla fine **Download CSV** salva la
tabella. Modello Whisper, modello VAD (Silero) e — se manca — una build
statica di ffmpeg vengono scaricati automaticamente al primo avvio.

Il log in tempo reale mostra ogni passo; in caso di errore la variante viene
marcata `error` con il messaggio completo e il benchmark prosegue con le
varianti successive (utile per capire se una GPU non è utilizzabile).

## Build locale

```bash
cd apps/whisper-bench/src-tauri
npx @tauri-apps/cli@^2 build                       # cpu-only
npx @tauri-apps/cli@^2 build -- --features vulkan  # cpu + vulkan
```

## Build CI

Workflow manuale `whisper-bench` (Actions → whisper-bench → Run workflow):
builda sempre linux/windows in variante cpu e vulkan, e opzionalmente
CUDA e ROCm (checkbox). Gli artifact (AppImage/deb/NSIS) sono scaricabili
dalla pagina del run e distribuibili ai tester.
