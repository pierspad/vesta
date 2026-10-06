<script lang="ts">
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { listen } from "@tauri-apps/api/event";
  import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { onMount, onDestroy, untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import { loadAndValidateApiKeys, type ApiKeyConfig } from "$lib/config/apiKeys";
  import {
    loadTiers,
    tiersHaveUsableEntries,
    TIERS_UPDATED_EVENT,
    type Tier,
  } from "$lib/config/translationTiers";
  import {
    buildTiersPayload,
    checkTiersAvailability,
    countTiersAndEndpoints,
    tiersUnavailableMessage,
    type TierEntryPayload,
    type TiersUnavailableReason,
  } from "$lib/config/llmTiers";
  import { setupWebviewDragDrop } from "$lib/utils/dragDrop";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  import PathPickerField from "$lib/components/PathPickerField.svelte";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import { loadRefinementPrompt } from "$lib/config/refinementPrompt";
  import FooterActions from "$lib/components/FooterActions.svelte";
  import Card from "$lib/components/Card.svelte";
  import SectionHeader from "$lib/components/SectionHeader.svelte";

  interface Props {
    active?: boolean;
    onGoToSettings?: (section: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightId?: string) => void;
  }

  let { active = true, onGoToSettings }: Props = $props();

  let llmError = $state<TiersUnavailableReason | null>(null);
  let isValidatingLlm = $state(false);
  let llmHighlightPulse = $state(false);
  let pulseTimer: ReturnType<typeof setTimeout> | null = null;
  let validationCheckCount = 0;

  function triggerLlmHighlight() {
    llmHighlightPulse = true;
    if (pulseTimer) clearTimeout(pulseTimer);
    pulseTimer = setTimeout(() => {
      llmHighlightPulse = false;
    }, 1800);
  }

  let t = $derived($locale);

  interface RefineCard {
    id: string;
    expression: string;
    meaning: string;
    notes: string;
    initialNotes?: string;
  }

  let filePath = $state("");
  let fileName = $derived(getFileName(filePath));
  let fileExtension = $derived(filePath.split(".").pop()?.toUpperCase() || "APKG");
  let cards = $state<RefineCard[]>([]);
  let selectedCardIndex = $state<number | null>(null);
  let searchQuery = $state("");
  let mode = $state<"manual" | "auto">("manual");
  let isLoading = $state(false);
  let isSaving = $state(false);

  // Automatic refinement state
  let autoRefining = $state(false);
  let progressCurrent = $state(0);
  let progressTotal = $state(0);
  let logs = $state<string[]>([]);
  let runSummary = $state<RefineRunSummary | null>(null);
  let progressFailed = $state(0);
  let isStopping = $state(false);
  const MAX_LOGS = 150;
  function addLog(message: string) { logs = [message, ...logs].slice(0, MAX_LOGS); }
  let customPrompt = $state("");
  let onlyUnannotated = $state(true);
  let useBatchMode = $state(true);
  let isSingleRefining = $state(false);

  // States to track card-specific generation activity
  let singleRefiningCardIds = $state<string[]>([]);
  let autoRefineGroupCardIds = new SvelteSet<string>();

  let annotatedCount = $derived(
    cards.filter((c) => c.notes.trim() !== "").length
  );
  let modifiedCount = $derived(
    cards.filter((c) => c.notes !== (c.initialNotes || "")).length
  );

  $effect(() => {
    aiStore.autoRefining = autoRefining;
    aiStore.isSingleRefining = isSingleRefining;
  });

  $effect(() => {
    if (aiStore.killSwitchActive) {
      if (autoRefining || isSingleRefining) void stopAutoRefinement();
      mode = "manual";
    }
  });

  const isCardRefining = (cardId: string) => {
    return singleRefiningCardIds.includes(cardId) || autoRefineGroupCardIds.has(cardId);
  };

  function setMode(next: "manual" | "auto") {
    if (aiStore.killSwitchActive) return;
    mode = next;
    if (mode === "auto") {
      refreshLlmConfig();
    }
  }

  // LLM tiers
  let tiers = $state<Tier[]>([]);
  let apiKeys = $state<ApiKeyConfig[]>([]);
  let useTiers = $derived(tiersHaveUsableEntries(tiers));
  let tierCounts = $derived(countTiersAndEndpoints(tiers));

  let cardIndices = $derived(new Map(cards.map((card, index) => [card.id, index])));
  let normalizedSearch = $derived(searchQuery.trim().toLowerCase());
  let filteredCards = $derived(
    cards.filter(
      (c) =>
        c.expression.toLowerCase().includes(normalizedSearch) ||
        c.meaning.toLowerCase().includes(normalizedSearch) ||
        c.notes.toLowerCase().includes(normalizedSearch)
    )
  );

  let operationBusy = $derived(isLoading || isSaving || autoRefining || isSingleRefining);
  let selectedFilteredIndex = $derived(filteredCards.findIndex((c) => c === selectedCard));
  function navigateCard(direction: -1 | 1) {
    const next = filteredCards[selectedFilteredIndex + direction];
    if (next) selectedCardIndex = cards.indexOf(next);
  }

  let selectedCard = $derived(
    selectedCardIndex !== null && cards[selectedCardIndex]
      ? cards[selectedCardIndex]
      : null
  );

  // Reconcile selection when the user changes the search, without moving
  // the editor away while its own Notes value is being edited.
  $effect(() => {
    searchQuery;
    untrack(() => {
      if (!selectedCard || !filteredCards.includes(selectedCard)) {
        selectedCardIndex = filteredCards.length ? cards.indexOf(filteredCards[0]) : null;
      }
    });
  });

  let notesProxy = {
    get value() {
      if (selectedCardIndex !== null && cards[selectedCardIndex]) {
        return cards[selectedCardIndex].notes;
      }
      return "";
    },
    set value(val: string) {
      if (selectedCardIndex !== null && cards[selectedCardIndex]) {
        cards[selectedCardIndex].notes = val;
      }
    }
  };

  let hasUnsavedChanges = $derived(
    cards.length > 0 && cards.some((c) => c.notes !== (c.initialNotes || ""))
  );
  let showOverwriteConfirm = $state(false);
  let pendingPathToLoad = $state<string | null>(null);

  async function triggerLoadFile(path: string) {
    if (operationBusy) return;
    if (hasUnsavedChanges) {
      pendingPathToLoad = path;
      showOverwriteConfirm = true;
    } else {
      await loadFile(path);
    }
  }

  // Drag & drop state
  let isDraggingOver = $state(false);

  function refreshLlmConfig() {
    tiers = loadTiers();
    apiKeys = loadAndValidateApiKeys();
    customPrompt = loadRefinementPrompt();
    void updateLlmStatus();
  }

  async function updateLlmStatus() {
    const currentCheckId = ++validationCheckCount;
    isValidatingLlm = true;
    try {
      const check = await checkTiersAvailability(tiers, apiKeys);
      if (currentCheckId !== validationCheckCount) return;
      llmError = check.available ? null : check.reason;
    } finally {
      if (currentCheckId === validationCheckCount) {
        isValidatingLlm = false;
      }
    }
  }

  function llmErrorMessage(reason: TiersUnavailableReason): string {
    return tiersUnavailableMessage(reason);
  }

  function refineTiersPayload(): TierEntryPayload[][] | null {
    if (!useTiers) return null;
    return buildTiersPayload(tiers, apiKeys);
  }

  $effect(() => {
    if (active) {
      untrack(() => {
        refreshLlmConfig();
      });
      const cleanupDragDrop = setupWebviewDragDrop({
        setDraggingOver: (v) => (isDraggingOver = v),
        onDrop: (paths) => {
          const path = paths[0];
          if (path && (/\.(apkg|tsv)$/i.test(path))) {
            void triggerLoadFile(path);
          } else {
            snackbar.show(t("refine.msg.unsupportedFormat"), "error");
          }
        },
        onError: (e) => console.warn("Failed to set up drag-drop listener in RefineTab:", e),
      });

      return () => {
        cleanupDragDrop();
      };
    }
  });

  onMount(() => {
    refreshLlmConfig();
    window.addEventListener(TIERS_UPDATED_EVENT, refreshLlmConfig);
    window.addEventListener("apikeys-updated", refreshLlmConfig);
  });

  onDestroy(() => {
    if (pulseTimer) clearTimeout(pulseTimer);
    if (autoRefining || isSingleRefining) void invoke("refine_cancel").catch(() => {});
    window.removeEventListener(TIERS_UPDATED_EVENT, refreshLlmConfig);
    window.removeEventListener("apikeys-updated", refreshLlmConfig);
  });

  async function selectFile() {
    try {
      const selected = await guardedOpen({
        filters: [
          { name: "Anki Deck (.apkg) / TSV (.tsv)", extensions: ["apkg", "tsv"] },
        ],
      });
      if (selected && typeof selected === "string") {
        await triggerLoadFile(selected);
      }
    } catch (err: any) {
      snackbar.show(err.toString(), "error");
    }
  }

  async function loadFile(path: string) {
    if (operationBusy) return;
    isLoading = true;
    try {
      const res = await invoke<RefineCard[]>("refine_load_file", { path });
      cards = res.map((c) => ({ ...c, initialNotes: c.notes }));
      filePath = path;
      searchQuery = "";
      progressCurrent = 0;
      progressTotal = 0;
      logs = [];
      runSummary = null;
      progressFailed = 0;
      selectedCardIndex = res.length > 0 ? 0 : null;
      singleRefiningCardIds = [];
      autoRefineGroupCardIds.clear();
      autoRefining = false;
      snackbar.show(
        t("refine.msg.loadSuccess", { count: res.length }),
        "success"
      );
    } catch (err: any) {
      snackbar.show(err.toString(), "error");
    } finally {
      isLoading = false;
    }
  }

  async function overwriteOriginalFile() {
    if (operationBusy || cards.length === 0 || !filePath) return;
    isSaving = true;
    try {
      const updates = cards.map((c) => ({ id: c.id, notes: c.notes }));
      const success = await invoke<boolean>("refine_save_file", {
        inputPath: filePath,
        outputPath: filePath,
        updates,
      });

      if (success) {
        snackbar.show(t("refine.msg.overwriteSuccess"), "success");
        cards = cards.map((c) => ({ ...c, initialNotes: c.notes }));
      } else {
        snackbar.show(t("refine.msg.overwriteError"), "error");
      }
    } catch (err: any) {
      snackbar.show(err.toString(), "error");
    } finally {
      isSaving = false;
    }
  }

  async function saveNewFileWithExtension(ext: "apkg" | "tsv") {
    if (operationBusy || cards.length === 0) return;
    isSaving = true;
    const selectionBeforeSave = selectedCardIndex;
    try {
      const currentExt = filePath.split(".").pop()?.toLowerCase() || "apkg";
      let defaultName = fileName || `refined_deck.${ext}`;
      if (fileName) {
        if (fileName.toLowerCase().endsWith(`.${currentExt}`)) {
          defaultName = fileName.substring(0, fileName.length - currentExt.length - 1) + `_refined.${ext}`;
        } else {
          defaultName = `${fileName}_refined.${ext}`;
        }
      }

      const selected = await guardedSave({
        defaultPath: defaultName,
        filters: [
          { name: ext === "tsv" ? "TSV (.tsv)" : "Anki Deck (.apkg)", extensions: [ext] },
        ],
      });

      if (selected && typeof selected === "string") {
        isSaving = true;
        const updates = cards.map((c) => ({ id: c.id, notes: c.notes }));
        const success = await invoke<boolean>("refine_save_file", {
          inputPath: filePath,
          outputPath: selected,
          updates,
        });

        if (success) {

          // TSV exports use row IDs instead of the original Anki note IDs.
          const savedCards = await invoke<RefineCard[]>("refine_load_file", { path: selected });
          filePath = selected;
          cards = savedCards.map((c) => ({ ...c, initialNotes: c.notes }));
          selectedCardIndex = cards.length > 0 ? Math.min(selectionBeforeSave ?? 0, cards.length - 1) : null;
          searchQuery = "";
          snackbar.show(t("refine.msg.saveSuccess"), "success");
        } else {
          snackbar.show(t("refine.msg.saveError"), "error");
        }
      }
    } catch (err: any) {
      snackbar.show(err.toString(), "error");
    } finally {
      isSaving = false;
    }
  }

  async function refineSingleCardAI() {
    if (operationBusy || aiStore.killSwitchActive || isValidatingLlm || selectedCardIndex === null || cards.length === 0) return;

    if (llmError) {
      triggerLlmHighlight();
      snackbar.show(llmErrorMessage(llmError), "error");
      return;
    }
    const tiersPayload = refineTiersPayload();
    if (!tiersPayload) {
      triggerLlmHighlight();
      snackbar.show(llmErrorMessage("noneConfigured"), "error");
      return;
    }

    const card = cards[selectedCardIndex];
    isSingleRefining = true;
    singleRefiningCardIds = [...singleRefiningCardIds, card.id];
    try {
      const response = await invoke<string>("refine_card_llm_tiered", {
        card: {
          id: card.id,
          expression: card.expression,
          meaning: card.meaning,
          notes: card.notes,
        },
        prompt: customPrompt,
        tiers: tiersPayload,
      });

      card.notes = response.trim();
      snackbar.show(t("refine.msg.generateSuccess"), "success");
    } catch (err: any) {
      const message = err?.toString() ?? "";
      if (message.includes("ERR_CANCELLED")) {
        snackbar.show(t("refine.log.stopped"), "info");
      } else if (message.includes("ERR_ALREADY_RUNNING")) {
        snackbar.show(t("common.error.alreadyRunning"), "error");
      } else {
        snackbar.show(t("refine.msg.generateError", { error: message }), "error");
      }
    } finally {
      singleRefiningCardIds = singleRefiningCardIds.filter((id) => id !== card.id);
      isSingleRefining = false;
    }
  }

  type RefineProgressPayload =
    | { type: "cardDone"; id: string; notes: string; done: number; total: number }
    | { type: "cardFailed"; id: string; error: string }
    | { type: "info"; message: string };

  interface RefineRunSummary {
    done: number;
    failed: number;
    remaining: number;
    poolExhausted: boolean;
    cancelled: boolean;
  }

  async function startAutoRefinement() {
    if (operationBusy || cards.length === 0) return;
    if (aiStore.killSwitchActive || isValidatingLlm) return;

    if (llmError) {
      triggerLlmHighlight();
      snackbar.show(llmErrorMessage(llmError), "error");
      return;
    }
    const tiersPayload = refineTiersPayload();
    if (!tiersPayload) {
      triggerLlmHighlight();
      snackbar.show(llmErrorMessage("noneConfigured"), "error");
      return;
    }

    const cardsToProcess = onlyUnannotated
      ? cards.filter((c) => !c.notes || c.notes.trim() === "")
      : cards;

    if (cardsToProcess.length === 0) {
      snackbar.show(t("refine.msg.noCardsToProcess"), "info");
      return;
    }

    autoRefining = true;
    runSummary = null;
    progressFailed = 0;
    isStopping = false;
    autoRefineGroupCardIds.clear();
    for (const card of cardsToProcess) autoRefineGroupCardIds.add(card.id);
    progressTotal = cardsToProcess.length;
    progressCurrent = 0;
    const endpointCount = tiersPayload.reduce((sum, tier) => sum + tier.length, 0);
    logs = [
      t("tiers.logActive", { tiers: tiersPayload.length, endpoints: endpointCount }),
      t("refine.log.cardsToProcess", { count: progressTotal }),
      t("refine.log.startAuto"),
    ];

    const runId = crypto.randomUUID();
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listen<{ runId: string; event: RefineProgressPayload }>("refine-progress", (event) => {
        if (event.payload.runId !== runId) return;
        const p = event.payload.event;
        if (p.type === "cardDone") {
          const idx = cardIndices.get(p.id) ?? -1;
          if (idx !== -1) {
            cards[idx].notes = p.notes;
            autoRefineGroupCardIds.delete(p.id);
            addLog(t("refine.log.success", { text: cards[idx].expression.substring(0, 30) }));
          }
          progressCurrent = Math.max(progressCurrent, p.done);
        } else if (p.type === "cardFailed") {
          autoRefineGroupCardIds.delete(p.id);
          const idx = cardIndices.get(p.id) ?? -1;
          const text = idx !== -1 ? cards[idx].expression.substring(0, 20) : p.id;
          progressFailed += 1;
          addLog(t("refine.log.error", { text, error: p.error }));
        } else {
          addLog(p.message);
        }
      });

      const summary = await invoke<RefineRunSummary>("refine_cards_llm_tiered", {
        cards: cardsToProcess.map((c) => ({
          id: c.id,
          expression: c.expression,
          meaning: c.meaning,
          notes: c.notes,
        })),
        prompt: customPrompt,
        tiers: tiersPayload,
        batchMode: useBatchMode,
        runId,
      });

      runSummary = summary;
      progressCurrent = summary.done;
      progressFailed = summary.failed;
      const message = t("refine.progress.summary", { success: summary.done, failed: summary.failed, remaining: summary.remaining });
      addLog(message);
      if (summary.cancelled) addLog(t("refine.log.stopped"));
      if (summary.poolExhausted) addLog(t("refine.msg.poolExhausted"));
      snackbar.show(message, summary.failed > 0 || summary.poolExhausted ? "error" : summary.cancelled ? "info" : "success");

    } catch (err: any) {
      const message = err?.toString() ?? "";
      if (message.includes("ERR_ALREADY_RUNNING")) {
        snackbar.show(t("common.error.alreadyRunning"), "error");
      } else {
        snackbar.show(t("refine.msg.generateError", { error: message }), "error");
      }
    } finally {
      unlisten?.();
      autoRefining = false;
      isStopping = false;
      autoRefineGroupCardIds.clear();
    }
  }

  async function stopAutoRefinement() {
    if (isStopping) return;
    isStopping = true;
    try {
      await invoke("refine_cancel");
    } catch (error) {
      isStopping = false;
      snackbar.show(String(error), "error");
    } finally {
      if (!autoRefining && !isSingleRefining) isStopping = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!active || mode !== "manual") return;
    if (selectedCardIndex === null || cards.length === 0) return;
    if (document.activeElement?.tagName === "TEXTAREA" || document.activeElement?.tagName === "INPUT") {
      if (e.key === "Escape") {
        (document.activeElement as HTMLElement).blur();
      }
      return;
    }

    if (e.key === "ArrowDown" || e.key === "j") {
      e.preventDefault();
      navigateCard(1);
    } else if (e.key === "ArrowUp" || e.key === "k") {
      e.preventDefault();
      navigateCard(-1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      const textarea = document.getElementById("card-notes") as HTMLTextAreaElement | null;
      if (textarea) {
        textarea.focus();
        textarea.setSelectionRange(textarea.value.length, textarea.value.length);
      }
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "copy";
    }
    isDraggingOver = true;
  }

  function handleDragLeave(e: DragEvent) {
    const rt = e.relatedTarget as HTMLElement | null;
    const ct = e.currentTarget as HTMLElement;
    if (rt && ct.contains(rt)) return;
    isDraggingOver = false;
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDraggingOver = false;

    if (e.dataTransfer && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      const path = (file as any).path;
      if (path && (/\.(apkg|tsv)$/i.test(path))) {
        await triggerLoadFile(path);
      } else {
        snackbar.show(t("refine.msg.unsupportedFormat"), "error");
      }
    }
  }

  onMount(() => {
    const load = (path: string) => { void triggerLoadFile(path); };
    (window as any).loadRefineFile = load;
    return () => {
      if ((window as any).loadRefineFile === load) delete (window as any).loadRefineFile;
    };
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

<!-- Legacy Panel-Style Layout Container -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="h-full flex flex-col bg-gray-900 relative overflow-hidden text-gray-100"
  ondragover={handleDragOver}
  ondrop={handleDrop}
  ondragleave={handleDragLeave}
>
  {#if isDraggingOver}
    <div
      class="absolute inset-0 z-50 bg-rose-500/10 border-rose-400/80 text-rose-400 border-2 border-dashed rounded-2xl flex items-center justify-center pointer-events-none"
    >
      <div class="text-center">
        <svg
          class="w-16 h-16 mx-auto mb-3 text-rose-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
          />
        </svg>
        <p class="text-lg font-medium text-rose-300">
          {t("refine.dropFileHere")}
        </p>
        <p class="text-sm text-gray-400 mt-1">{t("refine.dropFileHint")}</p>
      </div>
    </div>
  {/if}

  <!-- Main Workspace divided into panels -->
  <div class="flex-1 overflow-hidden p-6 min-h-0">
    <div class="flex flex-col gap-6 h-full">

      <!-- 1. Files & Output Panel (Standard Vesta Panel) -->
      <div class="glass-card p-5 shrink-0">
        <h3 class="panel-title-files-output mb-4 flex items-center gap-2 text-lg font-semibold">
          <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
          </svg>
          {t("common.filesAndOutput")}
        </h3>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 items-end">
          <!-- Input File Field -->
          <PathPickerField
            label={t('refine.deckLabel')}
            labelIcon="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
            required
            value={filePath || ""}
            placeholder={t('refine.deckPlaceholder')}
            browseTitle={t('refine.dropzone.browse')}
            onbrowse={selectFile}
            disabled={operationBusy}
          />

          <!-- Quick Deck Stats counters -->
          <div class="grid grid-cols-3 gap-3">
            <div class="bg-white/5 border border-white/10 rounded-xl p-2.5 text-center">
              <span class="block text-[10px] font-bold text-gray-500 uppercase tracking-wider">{t('refine.stats.totalCards')}</span>
              <span class="text-sm font-bold text-gray-200">{cards.length}</span>
            </div>
            <div class="bg-white/5 border border-white/10 rounded-xl p-2.5 text-center">
              <span class="block text-[10px] font-bold text-gray-500 uppercase tracking-wider">{t('refine.stats.annotated')}</span>
              <span class="text-sm font-bold text-rose-300">{annotatedCount}</span>
            </div>
            <div class="bg-white/5 border border-white/10 rounded-xl p-2.5 text-center">
              <span class="block text-[10px] font-bold text-gray-500 uppercase tracking-wider">{t('refine.stats.modified')}</span>
              <span class="text-sm font-bold text-amber-300">{modifiedCount}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 2. Main Grid: Left Column (Flashcards List) & Right Column (Workspace) -->
      <div class="grid grid-cols-1 md:grid-cols-12 gap-6 items-stretch flex-1 min-h-0">

        <!-- Left Column (5 cols): Flashcards List Panel -->
        <div class="md:col-span-5 glass-card p-5 flex flex-col min-h-0 overflow-hidden">
          <!-- Search Bar -->
          <div class="relative mb-3 shrink-0">
            <span class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-gray-400 z-10">
              <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
            </span>
            <input
              type="text"
              bind:value={searchQuery}
              placeholder={t('refine.searchPlaceholder')}
              class="input-modern w-full text-xs pr-4 py-2"
              style="padding-left: 2.5rem !important;"
            />
          </div>

          <!-- Scrollable Cards List (fills available vertical space) -->
          <div class="flex-1 min-h-0 overflow-y-auto [scrollbar-gutter:stable] p-2 bg-white/[0.03] border border-white/10 rounded-xl space-y-1.5 scrollbar-thin">
            {#if cards.length === 0}
              <div class="relative min-h-full">
                <div aria-hidden="true" class="space-y-1.5">
                  {#each Array(6) as _}
                    <div class="h-[88px] rounded-xl border border-white/5 bg-white/[0.02] p-3 space-y-2">
                      <div class="h-2 w-8 rounded bg-white/5"></div>
                      <div class="h-3 w-2/3 rounded bg-white/5"></div>
                      <div class="h-2 w-1/2 rounded bg-white/[0.03]"></div>
                    </div>
                  {/each}
                </div>
                <div class="absolute inset-0 flex items-center justify-center text-xs text-gray-400 italic">
                  <span class="rounded-lg bg-gray-900/90 px-4 py-2">{t('refine.noFlashcardsLoaded')}</span>
                </div>
              </div>
            {:else if filteredCards.length === 0}
              <div class="flex min-h-full items-center justify-center text-xs text-gray-400">{t("common.noResults")}</div>
            {:else}
              {#each filteredCards as card (card.id)}
                {@const isSelected = selectedCardIndex !== null && cards[selectedCardIndex]?.id === card.id}
                {@const globalIndex = cardIndices.get(card.id) ?? -1}
                <button
                  onclick={() => selectedCardIndex = globalIndex}
                  class="h-[88px] w-full text-left p-3 rounded-xl transition-all border flex flex-col gap-1 cursor-pointer
                    {isSelected
                      ? 'bg-rose-500/15 border-rose-500/40 text-white'
                      : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-gray-200'}"
                >
                  <div class="flex justify-between items-center text-[10px] text-gray-500">
                    <span class="font-mono">#{globalIndex + 1}</span>
                    <div class="flex items-center gap-1.5">
                      {#if isCardRefining(card.id)}
                        <span class="flex items-center gap-1 text-[9px] text-indigo-400 font-bold uppercase tracking-wider">
                          <svg class="animate-spin h-2.5 w-2.5 text-indigo-400" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                          </svg>
                          {t('refine.btn.generating')}
                        </span>
                      {/if}
                      {#if card.notes.trim() !== ""}
                        <span class="bg-rose-500/20 text-rose-300 px-1.5 py-0.5 rounded-full border border-rose-500/20 text-[9px] font-bold uppercase tracking-wider">{t('refine.badge.annotated')}</span>
                      {/if}
                      {#if card.notes !== (card.initialNotes || "")}
                        <span class="bg-amber-500/20 text-amber-300 px-1.5 py-0.5 rounded-full border border-amber-500/20 text-[9px] font-bold uppercase tracking-wider">{t('refine.badge.modified')}</span>
                      {/if}
                    </div>
                  </div>
                  <p class="text-xs font-semibold line-clamp-1 break-all text-gray-100 select-none">
                    {card.expression.replace(/<[^>]*>/g, "") || "—"}
                  </p>
                  <p class="text-[10px] line-clamp-1 break-all opacity-70 select-none">
                    {card.meaning.replace(/<[^>]*>/g, "") || "—"}
                  </p>
                </button>
              {/each}
            {/if}
          </div>
        </div>

        <!-- Right Column (7 cols): Refinement Panel -->
        <div class="md:col-span-7 glass-card p-5 flex flex-col min-h-0 overflow-hidden">
          {#snippet modeToggleSnippet()}
            {#if !aiStore.killSwitchActive}
              <div class="relative flex items-center p-1 bg-white/5 border border-white/10 rounded-xl w-[220px] ml-auto shrink-0 select-none">
                <div
                  class="absolute top-1 bottom-1 rounded-lg bg-indigo-600 shadow-md shadow-indigo-950/40 transition-all duration-300 ease-out pointer-events-none"
                  style="left: {mode === 'manual' ? '4px' : 'calc(50% + 2px)'}; width: calc(50% - 6px);"
                ></div>
                <button
                  type="button"
                  aria-pressed={mode === "manual"}
                  onclick={() => setMode("manual")}
                  class="relative z-10 flex-1 py-1 px-2.5 text-center text-xs transition-colors duration-200 cursor-pointer {mode === 'manual' ? 'text-white font-bold' : 'text-gray-400 hover:text-white font-semibold'}"
                >
                  {t('refine.mode.manual')}
                </button>
                <button
                  type="button"
                  aria-pressed={mode === "auto"}
                  onclick={() => setMode("auto")}
                  class="relative z-10 flex-1 py-1 px-2.5 text-center text-xs transition-colors duration-200 cursor-pointer {mode === 'auto' ? 'text-white font-bold' : 'text-gray-400 hover:text-white font-semibold'}"
                >
                  {t('refine.mode.auto')}
                </button>
              </div>
            {/if}
          {/snippet}

          <div class="flex items-center justify-between mb-4 shrink-0">
            <h3 class="text-lg font-semibold flex items-center gap-2 text-amber-400">
              <svg class="w-5 h-5 text-amber-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h14a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
              </svg>
              {t('refine.title')}
            </h3>
            {@render modeToggleSnippet()}
          </div>

          {#if mode === "manual"}
            <!-- MANUAL MODE: always visible; fields stay empty until a deck is loaded -->
              <div class="flex flex-col gap-4 flex-1 min-h-0 pt-3">
                <!-- Front / Back previews -->
                <div class="grid grid-cols-2 gap-3 shrink-0">
                  <div class="bg-white/5 border border-white/10 rounded-xl p-3 relative h-[68px] overflow-hidden">
                    <span class="absolute top-2 right-3 text-[9px] font-bold text-gray-500 uppercase tracking-wider">{t('refine.card.front')}</span>
                    <div class="text-xs font-semibold text-gray-200 mt-1 line-clamp-2">
                      {selectedCard?.expression.replace(/<[^>]*>/g, "") || "—"}
                    </div>
                  </div>

                  <div class="bg-white/5 border border-white/10 rounded-xl p-3 relative h-[68px] overflow-hidden">
                    <span class="absolute top-2 right-3 text-[9px] font-bold text-gray-500 uppercase tracking-wider">{t('refine.card.back')}</span>
                    <div class="text-xs font-semibold text-gray-200 mt-1 line-clamp-2">
                      {selectedCard?.meaning.replace(/<[^>]*>/g, "") || "—"}
                    </div>
                  </div>
                </div>

                <!-- Notes editor -->
                <div class="flex-1 flex flex-col min-h-0">
                  <div class="flex justify-between items-center mb-1.5 shrink-0">
                    <label for="card-notes" class="flex items-center gap-1.5 text-xs font-semibold text-gray-400">
                      <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                      </svg>
                      <span>{t('refine.notesLabel')}</span>
                    </label>
                    {#if isSingleRefining || autoRefining}
                      <button type="button" onclick={stopAutoRefinement} disabled={isStopping} class="text-xs text-red-300 px-3 py-1">{isStopping ? t("refine.btn.stopping") : t("refine.btn.stop")}</button>
                    {/if}
                    {#if !aiStore.killSwitchActive}
                      <button
                        type="button"
                        onclick={refineSingleCardAI}
                        disabled={operationBusy || isValidatingLlm || !!llmError || selectedCardIndex === null}
                        class="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-indigo-500/15 hover:bg-indigo-500/25 border border-indigo-500/30 hover:border-indigo-500/50 text-indigo-300 text-xs font-bold transition-all duration-200 cursor-pointer disabled:opacity-50"
                      >
                        {#if selectedCard && isCardRefining(selectedCard.id)}
                          <svg class="animate-spin h-3.5 w-3.5 text-indigo-300" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                          </svg>
                          {t('refine.btn.generating')}
                        {:else}
                          <svg class="w-3.5 h-3.5 text-indigo-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                          </svg>
                          {t('refine.btn.generateAI')}
                        {/if}
                      </button>
                    {/if}
                  </div>

                  <CodeEditor
                    id="card-notes"
                    bind:value={notesProxy.value}
                    readonly={isSaving || isLoading || selectedCardIndex === null || !!(selectedCard && isCardRefining(selectedCard.id))}
                    placeholder={selectedCardIndex === null ? t('refine.notesPlaceholderEmpty') : (!!(selectedCard && isCardRefining(selectedCard.id)) ? t('refine.notesPlaceholderGenerating') : t('refine.notesPlaceholder'))}
                    language="html"
                    heightClass="flex-1 min-h-[160px]"
                    textareaClass={selectedCardIndex === null || !!(selectedCard && isCardRefining(selectedCard.id)) ? 'opacity-70' : ''}
                  />
                </div>

                <!-- Navigation bar -->
                <div class="flex justify-between items-center text-xs text-gray-400 border-t border-white/5 pt-3 shrink-0">
                  <span class="flex items-center gap-1.5">
                    <kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">↑</kbd>/<kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">k</kbd> {t('refine.btn.prev')}
                    <span class="opacity-30 mx-1">•</span>
                    <kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">↓</kbd>/<kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">j</kbd> {t('refine.btn.next')}
                  </span>

                  <div class="flex items-center gap-2">
                    <button
                      onclick={() => navigateCard(-1)}
                      disabled={selectedFilteredIndex <= 0}
                      class="bg-white/5 hover:bg-white/10 disabled:opacity-30 border border-white/10 rounded-lg px-3 py-1.5 font-semibold transition-colors cursor-pointer"
                    >
                      {t('refine.btn.prev')}
                    </button>
                    <button
                      onclick={() => navigateCard(1)}
                      disabled={filteredCards.length === 0 || selectedFilteredIndex >= filteredCards.length - 1}
                      class="bg-white/5 hover:bg-white/10 disabled:opacity-30 border border-white/10 rounded-lg px-3 py-1.5 font-semibold transition-colors cursor-pointer"
                    >
                      {t('refine.btn.next')}
                    </button>
                  </div>
                </div>
              </div>
          {:else}
            <!-- AUTOMATIC MODE -->
            <div class="flex flex-col gap-4 flex-1 min-h-0 overflow-y-auto scrollbar-thin pt-3">
              <!-- LLM Tiers status card -->
              <button
                type="button"
                onclick={() => onGoToSettings?.("llm", "default-refinement-prompt")}
                class="flex flex-col gap-2 bg-white/5 border border-white/10 hover:bg-white/10 hover:border-white/20 rounded-xl p-3.5 transition-all duration-200 cursor-pointer text-left w-full shrink-0"
                class:llm-requirement-pulse={llmHighlightPulse}
              >
                <div class="flex items-center gap-2 text-xs text-gray-400">
                  <svg class="w-4 h-4 text-indigo-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                  </svg>
                  <span class="font-semibold text-gray-300">{t("refine.llmTiersLabel")}</span>
                  {#if useTiers}
                    <span class="text-[10px] bg-indigo-500/10 border border-indigo-500/20 text-indigo-300 px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ml-1">
                      {t("refine.llmTiersSummary", { tiers: tierCounts.tiers, endpoints: tierCounts.endpoints })}
                    </span>
                  {:else}
                    <span class="text-[10px] bg-white/5 border border-white/10 text-gray-500 px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ml-1">—</span>
                  {/if}
                </div>

                <div class="flex items-center gap-2 text-xs text-gray-400 flex-wrap">
                  <svg class="w-4 h-4 {llmError ? 'text-amber-400' : 'text-emerald-400'} shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                  </svg>
                  <span class="font-semibold text-gray-300">{t("refine.llmEngine")}</span>
                  {#if isValidatingLlm}
                    <span class="text-[10px] bg-white/5 border border-white/10 text-gray-400 px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ml-1">…</span>
                  {:else if !llmError}
                    <span class="text-[10px] bg-emerald-500/10 border border-emerald-500/20 text-emerald-300 px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ml-1">
                      {t("refine.llmTiersReady")}
                    </span>
                  {:else}
                    <span class="font-semibold text-amber-400 italic ml-1">{llmErrorMessage(llmError)}</span>
                  {/if}
                  <span class="text-[10px] text-gray-500 ml-auto">{t("refine.llmTiersEdit")}</span>
                </div>
              </button>

              <!-- Option cards -->
              <div class="grid grid-cols-2 gap-3 shrink-0">
                <button
                  type="button"
                  disabled={operationBusy}
                  aria-pressed={onlyUnannotated}
                  onclick={() => onlyUnannotated = !onlyUnannotated}
                  class="flex items-center justify-between p-3.5 rounded-xl border text-left transition-all duration-200 cursor-pointer select-none
                    {onlyUnannotated
                      ? 'bg-indigo-500/10 border-indigo-500/50 text-white'
                      : 'bg-white/5 border-white/10 text-gray-400 hover:bg-white/10 hover:text-gray-200'}"
                >
                  <span class="text-xs font-bold uppercase tracking-wider">{t('refine.options.onlyUnannotated')}</span>
                  <div class="w-3.5 h-3.5 rounded-full border border-indigo-400/50 flex items-center justify-center {onlyUnannotated ? 'bg-indigo-500' : ''}">
                    {#if onlyUnannotated}
                      <svg class="w-2.5 h-2.5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" /></svg>
                    {/if}
                  </div>
                </button>

                <button
                  type="button"
                  disabled={operationBusy}
                  aria-pressed={useBatchMode}
                  onclick={() => useBatchMode = !useBatchMode}
                  class="flex items-center justify-between p-3.5 rounded-xl border text-left transition-all duration-200 cursor-pointer select-none
                    {useBatchMode
                      ? 'bg-indigo-500/10 border-indigo-500/50 text-white'
                      : 'bg-white/5 border-white/10 text-gray-400 hover:bg-white/10 hover:text-gray-200'}"
                >
                  <span class="text-xs font-bold uppercase tracking-wider">{t('refine.options.batchMode')}</span>
                  <div class="w-3.5 h-3.5 rounded-full border border-indigo-400/50 flex items-center justify-center {useBatchMode ? 'bg-indigo-500' : ''}">
                    {#if useBatchMode}
                      <svg class="w-2.5 h-2.5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" /></svg>
                    {/if}
                  </div>
                </button>
              </div>

              <!-- Buttons -->
              <div class="flex items-center gap-3 mt-2 shrink-0">
                <button
                  onclick={() => onGoToSettings?.("llm", "default-refinement-prompt")}
                  class="flex-1 rounded-xl bg-white/5 hover:bg-white/10 border border-white/10 text-xs font-bold text-gray-300 px-4 py-2.5 transition-all cursor-pointer flex items-center justify-center gap-2"
                >
                  <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                  </svg>
                  {t('refine.btn.editPrompt')}
                </button>

                {#if autoRefining}
                  <button
                    onclick={stopAutoRefinement}
                    disabled={isStopping}
                    class="flex-1 rounded-xl bg-red-600/80 hover:bg-red-500/80 border border-red-500/30 text-xs font-bold text-red-100 px-4 py-2.5 transition-all cursor-pointer flex items-center justify-center gap-2"
                  >
                    {isStopping ? t('refine.btn.stopping') : t('refine.btn.stop')}
                  </button>
                {:else}
                  <button
                    onclick={startAutoRefinement}
                    disabled={operationBusy || isValidatingLlm || !!llmError || cards.length === 0}
                    class="flex-1 rounded-xl bg-amber-600 hover:bg-amber-500 border border-amber-500/30 disabled:bg-amber-600/40 text-xs font-bold text-white px-4 py-2.5 shadow-lg shadow-amber-950/30 transition-all cursor-pointer flex items-center justify-center gap-2 {(llmError || cards.length === 0) ? 'opacity-50 cursor-not-allowed' : ''}"
                  >
                    <svg class="w-4 h-4 text-amber-100" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                    </svg>
                    {t('refine.btn.startAI')}
                  </button>
                {/if}
              </div>

              <!-- Progress -->
                <div class="bg-white/5 border border-white/10 rounded-xl p-3 shrink-0">
                  <div class="flex justify-between text-xs text-gray-300 mb-1.5 font-semibold">
                    <span>{t('refine.progress.title')}</span>
                    <span>{progressCurrent + progressFailed} / {progressTotal} ({Math.round((progressTotal > 0 ? (progressCurrent + progressFailed) / progressTotal : 0) * 100)}%)</span>
                  </div>
                  <div class="w-full bg-white/10 h-2 rounded-full overflow-hidden">
                    <div
                      class="bg-gradient-to-r from-rose-500 to-pink-500 h-full rounded-full transition-all duration-300"
                      style="width: {(progressTotal > 0 ? (progressCurrent + progressFailed) / progressTotal : 0) * 100}%"
                    ></div>
                  </div>
                  {#if runSummary}
                    <p class="mt-2 text-xs text-gray-300" role="status">{t("refine.progress.summary", { success: runSummary.done, failed: runSummary.failed, remaining: runSummary.remaining })}</p>
                  {/if}
                </div>
              <details class="rounded-xl border border-white/10 bg-white/5 p-3 text-xs text-gray-400">
                <summary class="cursor-pointer">{t("refine.progress.activity")} ({logs.length})</summary>
                <div class="mt-2 h-24 overflow-y-auto [scrollbar-gutter:stable] break-words space-y-1">
                  {#each logs as log}<p>{log}</p>{/each}
                </div>
              </details>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <!-- Fixed Bottom Band with Action Buttons -->
  <FooterActions>
    {#snippet left()}
      <div class="flex flex-1 min-w-0 items-center gap-3">
      <div class="relative group shrink-0">
        <button
          onclick={overwriteOriginalFile}
          disabled={operationBusy || cards.length === 0}
          class="min-w-[155px] justify-center px-3 py-2.5 bg-rose-600 hover:bg-rose-500 disabled:bg-rose-600/40 text-white disabled:opacity-55 rounded-xl font-bold text-sm transition-all shadow-lg shadow-rose-950/30 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] cursor-pointer border border-rose-500/10"
        >
          {#if isSaving}
            <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {t('refine.action.saving')}
          {:else}
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
            </svg>
            {t('refine.action.overwrite', { extension: fileExtension })}
          {/if}
        </button>
        <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-rose-500/30 bg-gray-950/95 p-3 text-center text-xs text-rose-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {t('refine.action.tooltipOverwrite')}
        </div>
      </div>
      <div class="flex min-w-0 flex-1 max-w-[300px] items-center gap-1.5 text-xs text-gray-400 bg-white/5 border border-white/10 px-3 py-1.5 rounded-lg" title={filePath || undefined}>
        <span class="shrink-0 font-bold text-gray-300">{t('refine.action.fileLabel')}</span>
        <span class="min-w-0 truncate">{fileName || "—"}</span>
      </div>
      </div>
    {/snippet}
    {#snippet right()}
      <div class="flex shrink-0 items-center gap-2">
      <div class="relative group">
        <button
          onclick={() => saveNewFileWithExtension("apkg")}
          disabled={operationBusy || cards.length === 0 || fileExtension === "TSV"}
          class="min-w-[135px] justify-center px-3 py-2.5 bg-emerald-600 hover:bg-emerald-500 disabled:bg-emerald-600/55 disabled:opacity-55 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed cursor-pointer"
        >
          {#if isSaving}
            <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {t('refine.action.saving')}
          {:else}
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6M5 19V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2H7a2 2 0 01-2-2z" />
            </svg>
            {t('refine.action.saveAsApkg')}
          {/if}
        </button>
        <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-emerald-500/30 bg-gray-950/95 p-3 text-center text-xs text-emerald-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {fileExtension === "TSV"
            ? t('refine.action.cannotSaveTsvAsApkg')
            : t('refine.action.tooltipSaveAsApkg')}
        </div>
      </div>

      <div class="relative group">
        <button
          onclick={() => saveNewFileWithExtension("tsv")}
          disabled={operationBusy || cards.length === 0}
          class="min-w-[135px] justify-center px-3 py-2.5 bg-cyan-600 hover:bg-cyan-500 disabled:bg-cyan-600/55 disabled:opacity-55 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-cyan-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed cursor-pointer"
        >
          {#if isSaving}
            <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {t('refine.action.saving')}
          {:else}
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6M5 19V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2H7a2 2 0 01-2-2z" />
            </svg>
            {t('refine.action.saveAsTsv')}
          {/if}
        </button>
        <div class="pointer-events-none absolute bottom-full right-0 z-50 mb-3 rounded-xl border border-cyan-500/30 bg-gray-950/95 p-3 text-center text-xs text-cyan-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {t('refine.action.tooltipSaveAsTsv')}
        </div>
      </div>
      </div>
    {/snippet}
  </FooterActions>

  <ConfirmDialog
    show={showOverwriteConfirm}
    title={t('refine.warning.unsavedChangesTitle')}
    message={t('refine.warning.unsavedChangesMsg')}
    confirmText={t('refine.warning.confirmLoad')}
    cancelText={t('common.cancel')}
    variant="warning"
    on:cancel={() => {
      showOverwriteConfirm = false;
      pendingPathToLoad = null;
    }}
    on:confirm={async () => {
      showOverwriteConfirm = false;
      if (pendingPathToLoad) {
        await loadFile(pendingPathToLoad);
        pendingPathToLoad = null;
      }
    }}
  />
</div>

<style>
  :global(.llm-requirement-pulse) {
    animation: llm-requirement-pulse 0.9s ease-in-out 2;
    border-color: rgba(251, 191, 36, 0.75) !important;
    box-shadow:
      0 0 0 1px rgba(251, 191, 36, 0.3),
      0 0 24px rgba(251, 191, 36, 0.24) !important;
  }

  @keyframes llm-requirement-pulse {
    0%,
    100% {
      border-color: rgba(251, 191, 36, 0.35);
      box-shadow: 0 0 0 0 rgba(251, 191, 36, 0);
    }

    45% {
      border-color: rgba(251, 191, 36, 0.9);
      box-shadow:
        0 0 0 1px rgba(251, 191, 36, 0.45),
        0 0 28px rgba(251, 191, 36, 0.36);
    }
  }
</style>
