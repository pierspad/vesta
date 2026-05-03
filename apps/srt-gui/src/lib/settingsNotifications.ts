export const SETTINGS_ACTION_REQUIRED_KEY = "vesta-settings-action-required";
export const SETTINGS_ACTION_REQUIRED_EVENT = "vesta-settings-action-required-changed";
export const SETTINGS_ACTION_CURRENT_HASH_KEY = "vesta-settings-action-current-hash";
export const SETTINGS_ACTION_DISMISSED_HASH_KEY = "vesta-settings-action-dismissed-hash";

export type SettingsActionNotificationDetail = {
  required: boolean;
  hash: string;
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

export function isSettingsActionVisible(hash: string): boolean {
  if (!hash || typeof localStorage === "undefined") return false;
  return localStorage.getItem(SETTINGS_ACTION_DISMISSED_HASH_KEY) !== hash;
}

export function publishSettingsActionState(hash: string): boolean {
  if (typeof window === "undefined" || typeof localStorage === "undefined") return false;

  const required = isSettingsActionVisible(hash);
  localStorage.setItem(SETTINGS_ACTION_CURRENT_HASH_KEY, hash);
  localStorage.setItem(SETTINGS_ACTION_REQUIRED_KEY, String(required));
  window.dispatchEvent(
    new CustomEvent<SettingsActionNotificationDetail>(SETTINGS_ACTION_REQUIRED_EVENT, {
      detail: { required, hash },
    }),
  );
  return required;
}

export function getStoredSettingsActionState(): SettingsActionNotificationDetail {
  if (typeof localStorage === "undefined") return { required: false, hash: "" };
  const hash = localStorage.getItem(SETTINGS_ACTION_CURRENT_HASH_KEY) || "";
  return { required: isSettingsActionVisible(hash), hash };
}

export function markCurrentSettingsActionRead(): SettingsActionNotificationDetail {
  if (typeof localStorage === "undefined") return { required: false, hash: "" };
  const hash = localStorage.getItem(SETTINGS_ACTION_CURRENT_HASH_KEY) || "";
  if (hash) localStorage.setItem(SETTINGS_ACTION_DISMISSED_HASH_KEY, hash);
  const required = publishSettingsActionState(hash);
  return { required, hash };
}
