import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";
import { invokeCommand } from "$lib/services/tauriClient";
import type { TierEntryPayload } from "$lib/config/llmTiers";

export interface RefineAutoCard {
  id: string;
  expression: string;
  meaning: string;
  notes: string;
}

export type RefineProgressPayload =
  | { type: "cardDone"; id: string; notes: string; done: number; total: number }
  | { type: "cardFailed"; id: string; error: string }
  | { type: "info"; message: string };

export interface RefineRunSummary {
  done: number;
  failed: number;
  remaining: number;
  poolExhausted: boolean;
  cancelled: boolean;
}

interface RefineProgressEvent {
  runId: string;
  event: RefineProgressPayload;
}

export type RefineListen = <T>(name: string, handler: EventCallback<T>) => Promise<UnlistenFn>;

export interface RefineAutoRunOptions {
  cards: RefineAutoCard[];
  prompt: string;
  tiers: TierEntryPayload[][];
  batchMode: boolean;
  isCancelled: () => boolean;
  onProgress: (event: RefineProgressPayload) => void;
  subscribe?: RefineListen;
  invoke?: (command: string, payload: Record<string, unknown>) => Promise<RefineRunSummary>;
  createRunId?: () => string;
}

/** Owns the run ID and Tauri listener/invocation protocol for an automatic refine run. */
export async function runRefineAuto(options: RefineAutoRunOptions): Promise<RefineRunSummary | null> {
  const subscribe = options.subscribe ?? listen;
  const invoke = options.invoke ?? (async (command, payload) =>
    invokeCommand<RefineRunSummary>(command, payload));
  const runId = (options.createRunId ?? (() => crypto.randomUUID()))();
  let active = true;
  let unlisten: UnlistenFn | undefined;

  try {
    unlisten = await subscribe<RefineProgressEvent>("refine-progress", (event) => {
      if (active && event.payload.runId === runId) options.onProgress(event.payload.event);
    });

    // stopAutoRefinement may run while listen() is still resolving.
    if (options.isCancelled()) return null;

    return await invoke("refine_cards_llm_tiered", {
      cards: options.cards,
      prompt: options.prompt,
      tiers: options.tiers,
      batchMode: options.batchMode,
      runId,
    });
  } finally {
    active = false;
    unlisten?.();
  }
}
