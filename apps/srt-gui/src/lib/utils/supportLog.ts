export type SupportEntry = { time: string; kind: string; message: string };
let sink: ((entry: SupportEntry) => void) | null = null;
export function setSupportLogSink(value: typeof sink) { sink = value; }

export function redactLogText(value: string): string {
  return value
    .replace(/((?:api[_-]?key|authorization|access[_-]?token|refresh[_-]?token|password|secret|token)["']?\s*[:=]\s*)(["'])(.*?)\2/gi, '$1"[redacted]"')
    .replace(/\bBearer\s+[^\s"',}]+/gi, "Bearer [redacted]")
    .replace(/\b(?:sk-|AIza)[A-Za-z0-9_-]{12,}/g, "[redacted]")
    .replace(/((?:api[_-]?key|authorization|access[_-]?token|refresh[_-]?token|password|secret|token)["']?\s*[:=]\s*["']?)[^\s"',}&]+/gi, "$1[redacted]");
}
export function safeLogValue(value: unknown): string {
  if (value instanceof Error) return redactLogText(`${value.name}: ${value.message}\n${value.stack || ""}`).slice(0, 4000);
  if (typeof value === "string") return redactLogText(value).slice(0, 4000);
  try {
    return redactLogText(JSON.stringify(value, (key, item) =>
      /key|token|password|authorization|secret|subtitle|prompt|context/i.test(key) ? "[redacted]" : item,
    ) ?? String(value)).slice(0, 4000);
  } catch { return "[unserializable]"; }
}
export function recordSupportEvent(kind: string, message: string) {
  sink?.({ time: new Date().toISOString(), kind, message: redactLogText(message).slice(0, 4000) });
}
let installed = false;
export function installSupportLogging() {
  if (installed || typeof window === "undefined") return;
  installed = true;
  for (const level of ["log", "info", "warn", "error", "debug"] as const) {
    const original = console[level].bind(console);
    console[level] = (...values: unknown[]) => {
      recordSupportEvent(`console.${level}`, values.map(safeLogValue).join(" "));
      original(...values);
    };
  }
  window.addEventListener("error", event => recordSupportEvent("error", safeLogValue(event.error || event.message)));
  window.addEventListener("unhandledrejection", event => recordSupportEvent("error", safeLogValue(event.reason)));
  document.addEventListener("click", event => {
    const control = event.target instanceof Element ? event.target.closest("button, a, [role=tab], [role=switch]") : null;
    if (control) recordSupportEvent("ui.click", (control.getAttribute("aria-label") || control.getAttribute("title") || control.textContent || control.tagName).trim().slice(0, 160));
  }, true);
}
