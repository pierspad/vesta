import { mount } from "svelte";
import "./app.css";
import { hydrate as hydrateVestaConfig } from "$lib/config/vestaConfig";

// Prevent WebKit from handling dropped files (triggers GStreamer errors).
// Only intercept OS file drops (dataTransfer contains "Files").
// Must use capture phase to intercept before WebKit's native media handling.
for (const evt of ["dragenter", "dragover", "drop"] as const) {
  document.addEventListener(
    evt,
    (e) => {
      if ((e as DragEvent).dataTransfer?.types?.includes("Files")) {
        e.preventDefault();
        e.stopPropagation();
      }
    },
    { capture: true },
  );
}

// Keep global errors visible in logs without destroying the UI state.
window.onerror = (msg, src, line, col, err) => {
  console.error("[GlobalError]", {
    msg,
    src,
    line,
    col,
    stack: err?.stack,
  });
  return false;
};
window.onunhandledrejection = (e) => {
  console.error("[UnhandledRejection]", {
    reason: e.reason,
    stack: e.reason?.stack,
  });
};

try {
  // Idrata la cache di vesta_config.json PRIMA di importare App.svelte.
  // Molti store (aiStore, ankiStore, uiModeStore, i18n, ...) sono singleton
  // costruiti a livello di modulo -- il loro `$state(vestaConfig.getItem(...))`
  // gira nel momento in cui il modulo viene *importato*, non quando il
  // componente viene montato. Con un `import App from "./App.svelte"` statico
  // in cima al file, l'intero grafo di moduli di App verrebbe valutato prima
  // di questo `await` (gli import statici vengono issati ed eseguiti prima
  // del corpo del modulo corrente), leggendo la cache ancora vuota. L'import
  // dinamico qui sotto garantisce che App e tutti i suoi store vengano
  // valutati solo dopo che la cache è stata idratata.
  await hydrateVestaConfig();
  // Carica il dizionario della lingua iniziale (chunk on-demand) prima del
  // mount, così il primo paint è già nella lingua giusta. L'import dinamico
  // qui sotto valuta i18n DOPO l'idratazione della config (vedi sopra).
  const { initI18n } = await import("$lib/i18n");
  await initI18n();
  const { default: App } = await import("./App.svelte");

  const app = mount(App, {
    target: document.getElementById("app")!,
  });
  // @ts-ignore
  window.__app = app;
} catch (e: any) {
  const message = e instanceof Error ? `${e.message}\n${e.stack || ""}` : String(e);
  document.body.innerHTML = `<pre style="color:red;padding:2em;white-space:pre-wrap">MOUNT ERROR: ${message}</pre>`;
}

export default {};
