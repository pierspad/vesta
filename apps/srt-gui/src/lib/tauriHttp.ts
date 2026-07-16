import { invoke } from "@tauri-apps/api/core";

/**
 * Sostituto locale di `@tauri-apps/plugin-http`'s `fetch`, appoggiato al
 * comando Rust `http_fetch` (vedi apps/srt-gui/src-tauri/src/commands/net.rs).
 *
 * Quel plugin porta con sé un secondo stack reqwest/hyper/rustls indipendente
 * da quello già linkato dal workspace (usato da srt-translate): essendo
 * versioni major diverse cargo non le unifica, raddoppiando decine di crate
 * nel lockfile solo per queste poche richieste HTTP (controllo aggiornamenti,
 * discovery modelli LLM). Il comando Rust riusa il client del workspace e
 * applica la stessa allowlist di host che prima viveva in
 * `capabilities/default.json`.
 *
 * L'API ricalca il sottoinsieme della Fetch API già usato nel codice
 * (status/ok/url/headers.get/json/text), così i call site restano quasi
 * identici a prima.
 */

export interface TauriFetchOptions {
  method?: string;
  headers?: Record<string, string>;
  body?: string;
  /** "follow" (default) oppure "manual" per leggere Location senza seguirlo. */
  redirect?: "follow" | "manual";
  /** Timeout della richiesta in ms (default lato Rust: 15000). */
  timeoutMs?: number;
}

export interface TauriFetchResponse {
  ok: boolean;
  status: number;
  url: string;
  headers: { get(name: string): string | null };
  text(): Promise<string>;
  json(): Promise<unknown>;
}

interface HttpFetchResult {
  status: number;
  url: string;
  headers: [string, string][];
  body: string;
}

export async function fetch(
  url: string,
  options: TauriFetchOptions = {},
): Promise<TauriFetchResponse> {
  const result = await invoke<HttpFetchResult>("http_fetch", {
    req: {
      url,
      method: options.method ?? "GET",
      headers: options.headers ? Object.entries(options.headers) : [],
      body: options.body,
      redirect: options.redirect ?? "follow",
      timeoutMs: options.timeoutMs,
    },
  });

  const headerMap = new Map(
    result.headers.map(([name, value]) => [name.toLowerCase(), value]),
  );

  return {
    ok: result.status >= 200 && result.status < 300,
    status: result.status,
    url: result.url,
    headers: { get: (name: string) => headerMap.get(name.toLowerCase()) ?? null },
    text: async () => result.body,
    json: async () => JSON.parse(result.body),
  };
}
