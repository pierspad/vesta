import { SnapshotHistory } from "$lib/workflows/snapshotHistory";

export interface RefineNotesEntry { id: string; notes: string }
export interface RefineHistoryCard extends RefineNotesEntry {}

/** Keep document history limited to notes, restoring each change by stable card id. */
export function createRefineNotesHistory(
  readCards: () => RefineHistoryCard[],
  restoreNotes: (notesById: RefineNotesEntry[]) => void,
) {
  return new SnapshotHistory<RefineNotesEntry[]>(
    () => readCards().map(({ id, notes }) => ({ id, notes })),
    restoreNotes,
  );
}
