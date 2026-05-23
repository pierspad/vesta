# Modifiche recenti (in preparazione alla release)

* **Project Governance**: Modified `.cursorrules` to enforce immediate change logging in `list_of_things_changed.md` and release notes updates in `release-notes.md`.
* **UI Shell & Sidebar Layout**: Standardized padding of the first card/panel in Svelte tabs (`AlignTab`, `FlashcardsTab`, `SyncTab`, `LogPanel`) to `p-5` (or `px-5`) and unified heading icon markup (nesting SVG icons inside heading tags and updating tag names) to resolve vertical baseline offsets and ensure perfect visual continuity.
* **Keyboard Shortcuts**: Added global `Ctrl + PageUp` and `Ctrl + PageDown` keyboard shortcuts in `App.svelte` for quick tab switching/navigation.
* **Frontend Optimization & Boilerplate Reduction**: Established a central, Svelte 5 Rune-based `snackbarStore.ts` to manage toast notifications, rendering a single `<Snackbar>` globally in `App.svelte` and refactoring all 6 main tabs (`FlashcardsTab`, `SettingsTab`, `SyncTab`, `TranscribeTab`, `TranslateTab`, `AlignTab`) to eliminate duplicate local state variables, timers, and redundant component templates.
* **Consolidation of Shared Utilities**: Standardized helper functions (`getFileName`, `inferLanguageFromPath`, `getFlagForPath`) under `models.ts` and successfully removed identical duplicate function definitions in `AlignTab.svelte`, `FlashcardsTab.svelte`, and `SyncTab.svelte` to unify file parsing logic and decrease LoC.
* **Lifecycle Consolidation & Memory Leak Prevention**: Merged multiple duplicate `onMount` blocks in `SettingsTab.svelte` into a single unified block with clear lifecycle boundaries. Properly resolved a global `"whisper-model-updated"` event listener memory leak in `TranscribeTab.svelte`.
* **Race-Condition Protected Tauri Subscriptions**: Re-engineered all asynchronous Tauri event registrations (`listen` and `onDragDropEvent`) in `TranslateTab`, `TranscribeTab`, `AlignTab`, `SyncTab`, `SettingsTab`, and `FlashcardsTab` using a latch-guard pattern to automatically prevent event listener leaks when components unmount before subscriptions resolve.
* **Passive CPU & IPC Usage Optimization**: Configured Svelte `$effect` triggers in `TranslateTab.svelte` to stop background subtitle preview polling when the tab is inactive, preserving IPC and CPU cycles while the application is in an idle state.
* **Frontend Unification & Boilerplate Reduction**: Unified the `handleTabChange` and `changeTab` functions in `App.svelte` and corrected keydown listener references, eliminating duplicate handlers and centralizing navigation logic.
* **Reusable Component Consolidation**: Integrated the global `<InfoModal>` component into `AlignTab.svelte` and removed the inline manual dialog HTML/CSS structure, resulting in over 30 lines of code reduction (LoC) and guaranteeing complete UI and layout consistency.
* **Context Menu Keyboard Shortcuts Accessibility**:
  * Added visual keyboard shortcut labels (e.g. `H`, `S`, `R`, `C / Ctrl C`, `X / Ctrl X`, `V / Ctrl V`, `A / Ctrl A`) and global shortcut key interception to standard right-click and settings notification context menu items in `AppContextMenu.svelte`.
  * Added single-key keyboard bindings (`U`, `R`, `C`, `X`, `V`, `A`) and visual labels to the code editor context menu in `CodeEditor.svelte`.
  * Added visual keyboard shortcuts (`P`, `G`) and active key interception to the subtitle sync context menu in `SyncTab.svelte`.
  * Added visual keyboard shortcuts (`E`, `S`, `D / Del`) and implemented global window key capturing using `<svelte:window>` in `FlashcardsTab.svelte` when the episode list context menu is active.
  * Appended a context menu keyboard shortcut accessibility rule to `.cursorrules` to govern future context menu development.
* **Aggiornamento delle Regole di Sviluppo (.cursorrules)**:
  * Ricreato e aggiornato il file `.cursorrules` di Vesta integrandolo con le regole e i principi avanzati di Armonia Visiva UI/UX (CSS shimmer skeletons, layout validation, CSS logical properties, empty state guidelines, e ottimizzazioni GPU desktop Tauri).
* **Raffinatezze UI/UX e Allineamenti di Layout**:
  * **Shortcuts Tab**: Impostata la selezione delle categorie predefinita a nessuna categoria selezionata (array vuoto `[]` all'avvio) per mostrare tutte le scorciatoie di default.
  * **Feedback Registrazione**: Rimosso il banner grafico di successo locale e integrato con chiamate alla snackbar globale con durata impostata a 2.5 secondi.
  * **Pulsanti di Reset**: Aggiornato il testo del pulsante e del dialogo di reset da "Ripristina Tutte" a "Ripristina Predefiniti" (utilizzando la localizzazione `settings.resetDefaults`).
  * **Allineamento Spaziale delle Bande**: Modificata l'altezza della banda superiore (`h-[89px]`) e inferiore (`h-[92px]`) di `ShortcutsTab.svelte` per combaciare esattamente con i divisori della sidebar.
  * **Sidebar**: Centrate verticalmente le 5 voci principali di navigazione introducendo uno spacer flessibile superiore, ed incrementato lo spazio tra di esse (`space-y-3`) e il padding verticale dei singoli pulsanti (`py-2.5`) per occupare lo spazio in modo armonioso.
  * **Rimozione Tasto Indietro Ridondante**: Rimosso il pulsante "Indietro" interno alla pagina di configurazione delle impostazioni. Sincronizzato lo stato tramite prop `$bindable()` per far sì che il pulsante in alto a sinistra della sidebar gestisca in modo intelligente sia il ritorno alla panoramica impostazioni che l'uscita.



