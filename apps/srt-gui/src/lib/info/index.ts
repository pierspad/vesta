/**
 * Central registry of all help/info sections across the application.
 *
 * Each tab's sections map a panel ID to a pair of i18n keys:
 *   - titleKey: the modal title (short label)
 *   - contentKey: the HTML body with detailed help content
 *
 * Adding a new info panel:
 *   1. Add the i18n keys to the locale JSON files
 *   2. Add the section entry here
 *   3. Use <InfoModal sections={syncSections} ... /> in the tab component
 */

export interface InfoSection {
  titleKey: string;
  contentKey: string;
}

// ─── Translation Tab ──────────────────────────────────────
export const translateSections: Record<string, InfoSection> = {
  files: {
    titleKey: "translate.file",
    contentKey: "translate.filesHelp",
  },
  options: {
    titleKey: "translate.options",
    contentKey: "translate.optionsHelp",
  },
};

// ─── Synchronization Tab ──────────────────────────────────
export const syncSections: Record<string, InfoSection> = {
  wizard: {
    titleKey: "sync.wizard.title",
    contentKey: "sync.wizardHelp",
  },
  status: {
    titleKey: "sync.statusTitle",
    contentKey: "sync.statusHelp",
  },
  subtitleList: {
    titleKey: "sync.subtitleList",
    contentKey: "sync.subtitleListHelp",
  },
};

// ─── Transcribe Tab ───────────────────────────────────────
export const transcribeSections: Record<string, InfoSection> = {
  whisperModel: {
    titleKey: "transcribe.whisperModel",
    contentKey: "transcribe.whisperModelHelp",
  },
  options: {
    titleKey: "transcribe.options",
    contentKey: "transcribe.optionsHelp",
  },
  files: {
    titleKey: "transcribe.files",
    contentKey: "transcribe.filesHelp",
  },
};

// ─── Flashcards Tab ───────────────────────────────────────
export const flashcardsSections: Record<string, InfoSection> = {
  files: {
    titleKey: "flashcards.files",
    contentKey: "flashcards.filesHelp",
  },
  filesMovie: {
    titleKey: "flashcards.files",
    contentKey: "flashcards.filesHelpMovie",
  },
  subtitleOptions: {
    titleKey: "flashcards.subtitleOptions",
    contentKey: "flashcards.subtitleOptionsHelp",
  },
  audioClips: {
    titleKey: "flashcards.generateAudioClips",
    contentKey: "flashcards.generateAudioClipsHelp",
  },
  snapshots: {
    titleKey: "flashcards.generateSnapshots",
    contentKey: "flashcards.generateSnapshotsHelp",
  },
  videoClips: {
    titleKey: "flashcards.generateVideoClips",
    contentKey: "flashcards.generateVideoClipsHelp",
  },
  naming: {
    titleKey: "flashcards.naming",
    contentKey: "flashcards.namingHelp",
  },
  filters: {
    titleKey: "flashcards.filters",
    contentKey: "flashcards.filtersHelp",
  },
  contextLines: {
    titleKey: "flashcards.contextLines",
    contentKey: "flashcards.contextLinesHelp",
  },
  ankiFields: {
    titleKey: "flashcards.ankiFields",
    contentKey: "flashcards.ankiFieldsHelp",
  },
  exportFormat: {
    titleKey: "flashcards.exportFormat",
    contentKey: "flashcards.exportFormatHelp",
  },
  cpuCores: {
    titleKey: "flashcards.cpuCores",
    contentKey: "flashcards.cpuCoresHelp",
  },
};

// ─── Revision Tab ─────────────────────────────────────────
export const revisionSections: Record<string, InfoSection> = {
  help: {
    titleKey: "align.helpTitle",
    contentKey: "align.helpContent",
  },
};
