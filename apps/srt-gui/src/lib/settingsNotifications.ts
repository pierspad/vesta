export const SETTINGS_ACTION_REQUIRED_KEY = "vesta-settings-action-required";
export const SETTINGS_ACTION_REQUIRED_EVENT = "vesta-settings-action-required-changed";
export const SETTINGS_ACTION_CURRENT_HASH_KEY = "vesta-settings-action-current-hash";
export const SETTINGS_ACTION_HIDDEN_HASH_KEY = "vesta-settings-action-hidden-hash";
export const SETTINGS_ACTION_READ_HASH_KEY = "vesta-settings-action-read-hash";

export type SettingsActionNotificationDetail = {
  required: boolean;
  hash: string;
  hidden: boolean;
  read: boolean;
};

export function buildSettingsActionHash(actions: {
  needsWhisper: boolean;
  needsLlm: boolean;
}): string {
  const activeActions: string[] = [];
  if (actions.needsWhisper) activeActions.push("whisper-model-missing:v1");
  if (actions.needsLlm) activeActions.push("llm-default-unready:v1");
  return activeActions.join("|");
}

function parseActionHash(hash: string): string[] {
  return hash.split("|").map((item) => item.trim()).filter(Boolean);
}

function getHiddenActionSet(): Set<string> {
  if (typeof localStorage === "undefined") return new Set();
  return new Set(parseActionHash(localStorage.getItem(SETTINGS_ACTION_HIDDEN_HASH_KEY) || ""));
}

/** Check if a notification hash has actual issues to report */
export function isSettingsActionActive(hash: string): boolean {
  return !!hash;
}

/** Check if the notification has been hidden */
export function isSettingsActionHidden(hash: string): boolean {
  if (!hash || typeof localStorage === "undefined") return false;
  const activeActions = parseActionHash(hash);
  if (activeActions.length === 0) return false;
  const hiddenActions = getHiddenActionSet();
  return activeActions.every((action) => hiddenActions.has(action));
}

/** Check if the notification has been marked as read */
export function isSettingsActionRead(hash: string): boolean {
  if (!hash || typeof localStorage === "undefined") return false;
  return localStorage.getItem(SETTINGS_ACTION_READ_HASH_KEY) === hash;
}

/** A notification is visible if: it has an active hash AND is not hidden */
export function isSettingsActionVisible(hash: string): boolean {
  if (!hash || typeof localStorage === "undefined") return false;
  return !isSettingsActionHidden(hash);
}

export function publishSettingsActionState(hash: string): SettingsActionNotificationDetail {
  if (typeof window === "undefined" || typeof localStorage === "undefined") {
    return { required: false, hash: "", hidden: false, read: false };
  }

  const hasIssues = isSettingsActionActive(hash);
  const hidden = isSettingsActionHidden(hash);
  const read = isSettingsActionRead(hash);
  const required = hasIssues && !hidden;

  localStorage.setItem(SETTINGS_ACTION_CURRENT_HASH_KEY, hash);
  localStorage.setItem(SETTINGS_ACTION_REQUIRED_KEY, String(required));

  const detail: SettingsActionNotificationDetail = { required, hash, hidden, read };
  window.dispatchEvent(
    new CustomEvent<SettingsActionNotificationDetail>(SETTINGS_ACTION_REQUIRED_EVENT, {
      detail,
    }),
  );
  return detail;
}

export function getStoredSettingsActionState(): SettingsActionNotificationDetail {
  if (typeof localStorage === "undefined") return { required: false, hash: "", hidden: false, read: false };
  const hash = localStorage.getItem(SETTINGS_ACTION_CURRENT_HASH_KEY) || "";
  const hasIssues = isSettingsActionActive(hash);
  const hidden = isSettingsActionHidden(hash);
  const read = isSettingsActionRead(hash);
  return { required: hasIssues && !hidden, hash, hidden, read };
}

/** Hide all current notifications (they can be shown again) */
export function hideSettingsNotifications(): SettingsActionNotificationDetail {
  if (typeof localStorage === "undefined") return { required: false, hash: "", hidden: true, read: false };
  const hash = localStorage.getItem(SETTINGS_ACTION_CURRENT_HASH_KEY) || "";
  const hiddenActions = getHiddenActionSet();
  parseActionHash(hash).forEach((action) => hiddenActions.add(action));
  if (hiddenActions.size > 0) {
    localStorage.setItem(SETTINGS_ACTION_HIDDEN_HASH_KEY, [...hiddenActions].join("|"));
  }
  return publishSettingsActionState(hash);
}

/** Show notifications that were previously hidden */
export function showSettingsNotifications(): SettingsActionNotificationDetail {
  if (typeof localStorage === "undefined") return { required: false, hash: "", hidden: false, read: false };
  localStorage.removeItem(SETTINGS_ACTION_HIDDEN_HASH_KEY);
  const hash = localStorage.getItem(SETTINGS_ACTION_CURRENT_HASH_KEY) || "";
  return publishSettingsActionState(hash);
}

/** Mark the current notification as read/acknowledged */
export function markSettingsNotificationRead(): SettingsActionNotificationDetail {
  if (typeof localStorage === "undefined") return { required: false, hash: "", hidden: false, read: true };
  const hash = localStorage.getItem(SETTINGS_ACTION_CURRENT_HASH_KEY) || "";
  if (hash) localStorage.setItem(SETTINGS_ACTION_READ_HASH_KEY, hash);
  return publishSettingsActionState(hash);
}

/** Unmark the notification as read */
export function unmarkSettingsNotificationRead(): SettingsActionNotificationDetail {
  if (typeof localStorage === "undefined") return { required: false, hash: "", hidden: false, read: false };
  localStorage.removeItem(SETTINGS_ACTION_READ_HASH_KEY);
  const hash = localStorage.getItem(SETTINGS_ACTION_CURRENT_HASH_KEY) || "";
  return publishSettingsActionState(hash);
}

/** Legacy compat: kept for use where old API was called */
export function markCurrentSettingsActionRead(): SettingsActionNotificationDetail {
  return hideSettingsNotifications();
}
