import { redactLogText } from "./supportLog";

/** Readable rejection detail for the UI; diagnostic stacks stay in support logs. */
export function errorMessage(error: unknown, fallback = "Unknown error"): string {
  const message = typeof error === "string" ? error
    : error && typeof error === "object" && "message" in error && typeof error.message === "string"
      ? error.message : "";
  return redactLogText(message.trim() || fallback);
}
