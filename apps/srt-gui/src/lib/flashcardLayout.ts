// Column-layout system for FlashcardsTab: which panel ids exist, their
// persisted (user-editable) arrangement, and the responsive layout the UI
// falls back to once the host column collapses from 3 -> 2 -> 1. Pulled out
// of FlashcardsTab.svelte as pure logic with no component-lifecycle state,
// same split as noteTypes.ts/languages.ts.

export const PANEL_IDS = [
  "files",
  "audioClips",
  "snapshots",
  "videoClips",
  "cardFilters",
  "naming",
  "progressResult",
  "logs",
] as const;

export type PanelId = (typeof PANEL_IDS)[number];

export interface ColumnLayout {
  col1: PanelId[];
  col2: PanelId[];
  col3: PanelId[];
}

const MOVIE_LAYOUT_KEY = "vesta-flashcards-layout-v4";
const SERIES_LAYOUT_KEY = "vesta-flashcards-series-layout-v4";

const DEFAULT_LAYOUT: ColumnLayout = {
  col1: ["files"],
  col2: ["audioClips", "snapshots", "videoClips"],
  col3: ["naming", "cardFilters", "progressResult"],
};

const DEFAULT_SERIES_LAYOUT: ColumnLayout = {
  col1: ["files"],
  col2: ["audioClips", "snapshots", "videoClips"],
  col3: ["naming", "cardFilters", "progressResult"],
};

export function cloneLayout(layout: ColumnLayout): ColumnLayout {
  return {
    col1: [...layout.col1],
    col2: [...layout.col2],
    col3: [...layout.col3],
  };
}

export function loadLayout(): ColumnLayout {
  return cloneLayout(DEFAULT_LAYOUT);
}

export function loadSeriesLayout(): ColumnLayout {
  return cloneLayout(DEFAULT_SERIES_LAYOUT);
}

export function saveLayout(layout: ColumnLayout) {
  localStorage.setItem(MOVIE_LAYOUT_KEY, JSON.stringify(layout));
}

export function saveSeriesLayout(layout: ColumnLayout) {
  localStorage.setItem(SERIES_LAYOUT_KEY, JSON.stringify(layout));
}

// Responsive columns: auto-collapse from 3 -> 2 -> 1 based on available width.
export const PREFERRED_COLUMN_COUNT = 3;
const STACK_TO_ONE_COLUMN_WIDTH = 900;
const STACK_TO_TWO_COLUMNS_WIDTH = 1200;

export function computeEffectiveColumnCount(width: number): 1 | 2 | 3 {
  if (width < STACK_TO_ONE_COLUMN_WIDTH) return 1;
  if (width < STACK_TO_TWO_COLUMNS_WIDTH) return 2;
  return PREFERRED_COLUMN_COUNT;
}

export function computeEffectivePanelLayout(
  seriesMode: boolean,
  effectiveColumnCount: 1 | 2 | 3,
  easyMode: boolean,
): ColumnLayout {
  if (seriesMode) {
    if (effectiveColumnCount === 3) {
      // In easy mode (no expert mode), spread audio/snapshot/video across 3 columns
      if (easyMode) {
        return {
          col1: ["files", "audioClips"],
          col2: ["snapshots"],
          col3: ["videoClips", "progressResult"],
        };
      }
      return {
        col1: ["files", "naming", "audioClips", "snapshots"],
        col2: ["videoClips"],
        col3: ["cardFilters", "progressResult"],
      };
    }

    if (effectiveColumnCount === 2) {
      // Easy mode: skip 2-col, collapse to 1 col directly
      if (easyMode) {
        return {
          col1: ["files", "audioClips", "snapshots", "videoClips", "progressResult"],
          col2: [],
          col3: [],
        };
      }
      return {
        col1: ["files", "audioClips", "snapshots", "videoClips", "progressResult"],
        col2: ["naming", "cardFilters"],
        col3: [],
      };
    }

    return {
      col1: [
        "files",
        "naming",
        "audioClips",
        "snapshots",
        "videoClips",
        "cardFilters",
        "progressResult",
      ],
      col2: [],
      col3: [],
    };
  } else {
    if (effectiveColumnCount === 3) {
      return {
        col1: ["files", "naming"],
        col2: ["audioClips", "videoClips"],
        col3: ["snapshots", "cardFilters", "progressResult"],
      };
    }

    if (effectiveColumnCount === 2) {
      return {
        col1: ["files", "naming", "cardFilters"],
        col2: ["audioClips", "snapshots", "videoClips", "progressResult"],
        col3: [],
      };
    }

    return {
      col1: [
        "files",
        "naming",
        "audioClips",
        "snapshots",
        "videoClips",
        "cardFilters",
        "progressResult",
      ],
      col2: [],
      col3: [],
    };
  }
}
