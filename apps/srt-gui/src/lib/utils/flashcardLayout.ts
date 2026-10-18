/** Stable panel order. CSS changes placement without remounting audio players. */
export const SETTINGS_PANEL_IDS = ["audioClips", "snapshots", "videoClips", "cardFilters"] as const;
export type SettingsPanelId = (typeof SETTINGS_PANEL_IDS)[number];
export type PanelId = "files" | SettingsPanelId;

// Use the workspace width, rather than window width, so the sidebar is accounted for.
export function computeEffectiveColumnCount(width: number): 1 | 2 | 3 {
  if (width < 900) return 1;
  if (width < 1200) return 2;
  return 3;
}
