<script lang="ts">
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { onMount, onDestroy, untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import { errorMessage } from "$lib/utils/errorMessage";
  import { loadAndValidateApiKeys, type ApiKeyConfig } from "$lib/config/apiKeys";
  import { loadTiers, tiersHaveUsableEntries, TIERS_UPDATED_EVENT, type Tier } from "$lib/config/translationTiers";
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
  import WorkflowWorkspace from "$lib/components/WorkflowWorkspace.svelte";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import { guidanceStore } from "$lib/stores/guidanceStore.svelte";
  import { RequirementFeedback } from "$lib/utils/requirementFeedback.svelte";
  const requirements = new RequirementFeedback<"files">();
  import { loadRefinementPrompt } from "$lib/config/refinementPrompt";
  import { createRefineCardSearch } from "$lib/workflows/refineCardSearch";
  import { runRefineAuto, type RefineRunSummary } from "$lib/workflows/refineAutoRun";
  import { createRefineNotesHistory } from "$lib/workflows/refineNotesHistory";
  import { historyShortcutAction } from "$lib/utils/shortcuts";
  import RefineFilesPanel from "./refine/RefineFilesPanel.svelte";
  import RefineCardListPanel, { type RefineCard } from "./refine/RefineCardListPanel.svelte";
  import RefineManualPanel from "./refine/RefineManualPanel.svelte";
  import RefineAutoPanel from "./refine/RefineAutoPanel.svelte";
  import RefineFooterBar from "./refine/RefineFooterBar.svelte";

  interface Props {
    active?: boolean;
    onGoToSettings?: (section: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightId?: string) => void;
  }

  let { active = true, onGoToSettings }: Props = $props();
  let t = $derived($locale);

  let llmError = $state<TiersUnavailableReason | null>(null);
  let isValidatingLlm = $state(false);
  let disposed = false;
  let llmHighlightPulse = $state(false);
  let pulseTimer: ReturnType<typeof setTimeout> | null = null;
  let validationCheckCount = 0;

  function triggerLlmHighlight() {
    llmHighlightPulse = true;
    if (pulseTimer) clearTimeout(pulseTimer);
    pulseTimer = setTimeout(() => { llmHighlightPulse = false; }, 1800);
  }

  let filePath = $state("");
  let fileName = $derived(getFileName(filePath));
  let fileExtension = $derived(filePath.split(".").pop()?.toUpperCase() || "APKG");
  let cards = $state<RefineCard[]>([]);
  let historyRevision = $state(0);
  const documentHistory = createRefineNotesHistory(
    () => cards,
    (snapshot) => {
      const notesById = new Map(snapshot.map((card) => [card.id, card.notes]));
      cards = cards.map((card) => notesById.has(card.id) ? { ...card, notes: notesById.get(card.id)! } : card);
      historyRevision++;
    },
  );
  let canUndoDocument = $derived.by(() => { void historyRevision; return documentHistory.canUndo; });
  let canRedoDocument = $derived.by(() => { void historyRevision; return documentHistory.canRedo; });
  let selectedCardIndex = $state<number | null>(null);
  let searchQuery = $state("");
  let mode = $state<"manual" | "auto">("manual");
  let isLoading = $state(false);
  let isSaving = $state(false);

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
  let singleRefiningCardIds = $state<string[]>([]);
  let autoRefineGroupCardIds = new SvelteSet<string>();

  let annotatedCount = $derived(cards.filter((c) => c.notes.trim() !== "").length);
  let modifiedCount = $derived(cards.filter((c) => c.notes !== (c.initialNotes || "")).length);

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

  const isCardRefining = (cardId: string) =>
    singleRefiningCardIds.includes(cardId) || autoRefineGroupCardIds.has(cardId);

  function setMode(next: "manual" | "auto") {
    if (aiStore.killSwitchActive) return;
    mode = next;
    if (mode === "auto") refreshLlmConfig();
  }

  let tiers = $state<Tier[]>([]);
  let apiKeys = $state<ApiKeyConfig[]>([]);
  let useTiers = $derived(tiersHaveUsableEntries(tiers));
  let tierCounts = $derived(countTiersAndEndpoints(tiers));

  const refineCardSearch = createRefineCardSearch<RefineCard>();
  let cardSearchIndex = $derived(refineCardSearch.index(cards));
  let cardPlainTexts = $derived(cardSearchIndex.plainTexts);
  let cardIndices = $derived(cardSearchIndex.indices);
  let filteredCards = $derived(refineCardSearch.filter(cards, cardPlainTexts, searchQuery));

  const CARD_ITEM_HEIGHT = 88;
  const CARD_ITEM_GAP = 6;
  const ROW_TOTAL_HEIGHT = CARD_ITEM_HEIGHT + CARD_ITEM_GAP;
  const VIRTUAL_OVERSCAN = 6;

  let cardsContainerEl = $state<HTMLElement | null>(null);
  let cardListScrollTop = $state(0);
  let cardListViewportHeight = $state(600);

  let totalVirtualHeight = $derived(
    filteredCards.length > 0 ? filteredCards.length * ROW_TOTAL_HEIGHT - CARD_ITEM_GAP : 0
  );
  let virtualStartIndex = $derived.by(() => {
    if (filteredCards.length === 0) return 0;
    return Math.max(0, Math.floor(cardListScrollTop / ROW_TOTAL_HEIGHT) - VIRTUAL_OVERSCAN);
  });
  let virtualEndIndex = $derived.by(() => {
    if (filteredCards.length === 0) return 0;
    const countInView = Math.ceil((cardListViewportHeight || 600) / ROW_TOTAL_HEIGHT);
    return Math.min(filteredCards.length, Math.floor(cardListScrollTop / ROW_TOTAL_HEIGHT) + countInView + VIRTUAL_OVERSCAN);
  });
  let virtualVisibleCards = $derived(filteredCards.slice(virtualStartIndex, virtualEndIndex));
  let virtualTopOffset = $derived(virtualStartIndex * ROW_TOTAL_HEIGHT);

  function handleCardListScroll(e: Event) {
    cardListScrollTop = (e.currentTarget as HTMLElement).scrollTop;
  }

  function scrollToCardIndex(filteredIndex: number) {
    if (!cardsContainerEl || filteredIndex < 0) return;
    const itemTop = filteredIndex * ROW_TOTAL_HEIGHT;
    const itemBottom = itemTop + CARD_ITEM_HEIGHT;
    const currentScroll = cardsContainerEl.scrollTop;
    const vpHeight = cardListViewportHeight || 600;
    if (itemTop < currentScroll) {
      cardsContainerEl.scrollTop = itemTop;
      cardListScrollTop = itemTop;
    } else if (itemBottom > currentScroll + vpHeight) {
      const nextScroll = itemBottom - vpHeight;
      cardsContainerEl.scrollTop = nextScroll;
      cardListScrollTop = nextScroll;
    }
  }

  let operationBusy = $derived(isLoading || isSaving || autoRefining || isSingleRefining);
  let selectedCard = $derived(selectedCardIndex !== null && cards[selectedCardIndex] ? cards[selectedCardIndex] : null);
  let selectedFilteredIndex = $derived(filteredCards.findIndex((c) => c === selectedCard));

  function navigateCard(direction: -1 | 1) {
    const nextFilteredIdx = selectedFilteredIndex + direction;
    const next = filteredCards[nextFilteredIdx];
    if (next) {
      selectedCardIndex = cards.indexOf(next);
      scrollToCardIndex(nextFilteredIdx);
    }
  }

  $effect(() => {
    void searchQuery;
    if (cardsContainerEl) {
      cardsContainerEl.scrollTop = 0;
      cardListScrollTop = 0;
    }
    untrack(() => {
      if (!selectedCard || !filteredCards.includes(selectedCard)) {
        selectedCardIndex = filteredCards.length ? cards.indexOf(filteredCards[0]) : null;
      }
    });
  });

  let notesProxy = {
    get value() {
      return selectedCardIndex !== null && cards[selectedCardIndex] ? cards[selectedCardIndex].notes : "";
    },
    set value(val: string) {
      if (selectedCardIndex !== null && cards[selectedCardIndex]) {
        if (cards[selectedCardIndex].notes === val) return;
        documentHistory.schedule();
        historyRevision++;
        cards[selectedCardIndex].notes = val;
      }
    },
  };

  let hasUnsavedChanges = $derived(cards.length > 0 && cards.some((c) => c.notes !== (c.initialNotes || "")));
  let showOverwriteConfirm = $state(false);
  let pendingPathToLoad = $state<string | null>(null);
  let isDraggingOver = $state(false);

  async function triggerLoadFile(path: string) {
    if (operationBusy) return;
    if (hasUnsavedChanges) {
      pendingPathToLoad = path;
      showOverwriteConfirm = true;
    } else {
      await loadFile(path);
    }
  }

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
      if (currentCheckId === validationCheckCount) isValidatingLlm = false;
    }
  }

  const llmErrorMessage = (reason: TiersUnavailableReason) => tiersUnavailableMessage(reason);
  const refineTiersPayload = (): TierEntryPayload[][] | null => useTiers ? buildTiersPayload(tiers, apiKeys) : null;

  $effect(() => {
    if (!active) return;
    untrack(() => { refreshLlmConfig(); });
    return setupWebviewDragDrop({
      setDraggingOver: (v) => (isDraggingOver = v),
      onDrop: (paths) => {
        const path = paths[0];
        if (path && /\.(apkg|tsv)$/i.test(path)) void triggerLoadFile(path);
        else snackbar.show(t("refine.msg.unsupportedFormat"), "error");
      },
      onError: (e) => console.warn("Failed to set up drag-drop listener in RefineTab:", e),
    });
  });

  onMount(() => {
    refreshLlmConfig();
    window.addEventListener(TIERS_UPDATED_EVENT, refreshLlmConfig);
    window.addEventListener("apikeys-updated", refreshLlmConfig);
    const load = (path: string) => { void triggerLoadFile(path); };
    (window as any).loadRefineFile = load;
    return () => {
      if ((window as any).loadRefineFile === load) delete (window as any).loadRefineFile;
    };
  });

  onDestroy(() => {
    disposed = true;
    if (pulseTimer) clearTimeout(pulseTimer);
    if (autoRefining || isSingleRefining) void invoke("refine_cancel").catch(() => {});
    aiStore.autoRefining = false;
    aiStore.isSingleRefining = false;
    window.removeEventListener(TIERS_UPDATED_EVENT, refreshLlmConfig);
    window.removeEventListener("apikeys-updated", refreshLlmConfig);
    documentHistory.dispose();
  });

  async function selectFile() {
    try {
      const selected = await guardedOpen({ filters: [{ name: "APKG / TSV", extensions: ["apkg", "tsv"] }] });
      if (selected && typeof selected === "string") await triggerLoadFile(selected);
    } catch (err: any) { snackbar.show(errorMessage(err), "error"); }
  }

  async function loadFile(path: string) {
    if (operationBusy) return;
    isLoading = true;
    try {
      const res = await invoke<RefineCard[]>("refine_load_file", { path });
      cards = res.map((c) => ({ ...c, initialNotes: c.notes }));
      documentHistory.reset();
      historyRevision++;
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
      snackbar.show(t("refine.msg.loadSuccess", { count: res.length }), "success");
    } catch (err: any) { snackbar.show(errorMessage(err), "error"); }
    finally { isLoading = false; }
  }

  async function overwriteOriginalFile() {
    if (operationBusy || cards.length === 0 || !filePath) return;
    isSaving = true;
    try {
      const updates = cards.map((c) => ({ id: c.id, notes: c.notes }));
      const success = await invoke<boolean>("refine_save_file", { inputPath: filePath, outputPath: filePath, updates });
      if (success) {
        snackbar.show(t("refine.msg.overwriteSuccess"), "success");
        cards = cards.map((c) => ({ ...c, initialNotes: c.notes }));
      } else snackbar.show(t("refine.msg.overwriteError"), "error");
    } catch (err: any) { snackbar.show(errorMessage(err), "error"); }
    finally { isSaving = false; }
  }

  async function saveNewFileWithExtension(ext: "apkg" | "tsv") {
    if (operationBusy || cards.length === 0) return;
    isSaving = true;
    const selectionBeforeSave = selectedCardIndex;
    try {
      const currentExt = filePath.split(".").pop()?.toLowerCase() || "apkg";
      let defaultName = fileName || `refined_deck.${ext}`;
      if (fileName) {
        defaultName = fileName.toLowerCase().endsWith(`.${currentExt}`)
          ? fileName.substring(0, fileName.length - currentExt.length - 1) + `_refined.${ext}`
          : `${fileName}_refined.${ext}`;
      }
      const selected = await guardedSave({
        defaultPath: defaultName,
        filters: [{ name: ext === "tsv" ? "TSV (.tsv)" : "Anki Deck (.apkg)", extensions: [ext] }],
      });
      if (selected && typeof selected === "string") {
        isSaving = true;
        const updates = cards.map((c) => ({ id: c.id, notes: c.notes }));
        const success = await invoke<boolean>("refine_save_file", { inputPath: filePath, outputPath: selected, updates });
        if (success) {
          const savedCards = await invoke<RefineCard[]>("refine_load_file", { path: selected });
          filePath = selected;
          cards = savedCards.map((c) => ({ ...c, initialNotes: c.notes }));
          selectedCardIndex = cards.length > 0 ? Math.min(selectionBeforeSave ?? 0, cards.length - 1) : null;
          searchQuery = "";
          snackbar.show(t("refine.msg.saveSuccess"), "success");
        } else snackbar.show(t("refine.msg.saveError"), "error");
      }
    } catch (err: any) { snackbar.show(errorMessage(err), "error"); }
    finally { isSaving = false; }
  }

  async function refineSingleCardAI() {
    if (operationBusy || aiStore.killSwitchActive || isValidatingLlm || selectedCardIndex === null || cards.length === 0) return;
    if (llmError) {
      triggerLlmHighlight();
      snackbar.show(llmErrorMessage(llmError), "error", undefined, "settings");
      guidanceStore.guideToSettings({ sourceTab: "refine", section: "llm", targetId: "llm-tiers" });
      return;
    }
    const tiersPayload = refineTiersPayload();
    if (!tiersPayload) {
      triggerLlmHighlight();
      snackbar.show(llmErrorMessage("noneConfigured"), "error", undefined, "settings");
      guidanceStore.guideToSettings({ sourceTab: "refine", section: "llm", targetId: "llm-tiers" });
      return;
    }

    const card = cards[selectedCardIndex];
    isSingleRefining = true;
    singleRefiningCardIds = [...singleRefiningCardIds, card.id];
    try {
      const response = await invoke<string>("refine_card_llm_tiered", {
        card: { id: card.id, expression: card.expression, meaning: card.meaning, notes: card.notes },
        prompt: customPrompt,
        tiers: tiersPayload,
      });
      const nextNotes = response.trim();
      if (nextNotes !== card.notes) {
        documentHistory.push();
        historyRevision++;
        card.notes = nextNotes;
      }
      snackbar.show(t("refine.msg.generateSuccess"), "success");
    } catch (err: any) {
      const message = err?.toString() ?? "";
      if (message.includes("ERR_CANCELLED")) snackbar.show(t("refine.log.stopped"), "info");
      else if (message.includes("ERR_ALREADY_RUNNING")) snackbar.show(t("common.error.alreadyRunning"), "error");
      else snackbar.show(t("refine.msg.generateError", { error: message }), "error");
    } finally {
      singleRefiningCardIds = singleRefiningCardIds.filter((id) => id !== card.id);
      isSingleRefining = false;
    }
  }

  async function startAutoRefinement() {
    if (operationBusy || cards.length === 0 || aiStore.killSwitchActive || isValidatingLlm) return;
    if (llmError) {
      triggerLlmHighlight();
      snackbar.show(llmErrorMessage(llmError), "error", undefined, "settings");
      guidanceStore.guideToSettings({ sourceTab: "refine", section: "llm", targetId: "llm-tiers" });
      return;
    }
    const tiersPayload = refineTiersPayload();
    if (!tiersPayload) {
      triggerLlmHighlight();
      snackbar.show(llmErrorMessage("noneConfigured"), "error", undefined, "settings");
      guidanceStore.guideToSettings({ sourceTab: "refine", section: "llm", targetId: "llm-tiers" });
      return;
    }

    const cardsToProcess = onlyUnannotated ? cards.filter((c) => !c.notes || c.notes.trim() === "") : cards;
    if (cardsToProcess.length === 0) { snackbar.show(t("refine.msg.noCardsToProcess"), "info"); return; }

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

    let autoHistoryCaptured = false;
    try {
      const summary = await runRefineAuto({
        cards: cardsToProcess.map((c) => ({ id: c.id, expression: c.expression, meaning: c.meaning, notes: c.notes })),
        prompt: customPrompt,
        tiers: tiersPayload,
        batchMode: useBatchMode,
        isCancelled: () => disposed || isStopping || aiStore.killSwitchActive,
        onProgress: (p) => {
          if (disposed) return;
          if (p.type === "cardDone") {
            const idx = cardIndices.get(p.id) ?? -1;
            if (idx !== -1) {
              if (!autoHistoryCaptured && cards[idx].notes !== p.notes) {
                documentHistory.push();
                historyRevision++;
                autoHistoryCaptured = true;
              }
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
          } else addLog(p.message);
        },
      });
      if (!summary || disposed) return;
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
      if (!disposed) {
        if (message.includes("ERR_ALREADY_RUNNING")) snackbar.show(t("common.error.alreadyRunning"), "error");
        else snackbar.show(t("refine.msg.generateError", { error: message }), "error");
      }
    } finally {
      if (!disposed) { autoRefining = false; isStopping = false; autoRefineGroupCardIds.clear(); }
    }
  }

  function handleBlockedGenerateSingle() {
    if (operationBusy || aiStore.killSwitchActive || isValidatingLlm) return;
    if (llmError) {
      triggerLlmHighlight();
      snackbar.show(llmErrorMessage(llmError), "error", undefined, "settings");
      guidanceStore.guideToSettings({ sourceTab: "refine", section: "llm", targetId: "llm-tiers" });
      return;
    }
    if (selectedCardIndex === null || cards.length === 0) {
      requirements.pulse(["files"]);
      snackbar.show(t("refine.msg.noCardSelectedAlert"), "info");
    }
  }

  function handleBlockedStartAuto() {
    if (operationBusy || aiStore.killSwitchActive || isValidatingLlm) return;
    if (cards.length === 0) {
      requirements.pulse(["files"]);
      snackbar.show(t("refine.msg.noCardsToProcess"), "info");
      return;
    }
    if (llmError) {
      triggerLlmHighlight();
      snackbar.show(llmErrorMessage(llmError), "error", undefined, "settings");
      guidanceStore.guideToSettings({ sourceTab: "refine", section: "llm", targetId: "llm-tiers" });
    }
  }

  function handleBlockedSave(kind: "overwrite" | "apkg" | "tsv") {
    if (operationBusy) return;
    if (cards.length === 0) {
      requirements.pulse(["files"]);
      snackbar.show(t("refine.msg.noCardsToProcess"), "info");
      return;
    }
    if (kind === "apkg" && fileExtension === "TSV") {
      snackbar.show(t("refine.action.cannotSaveTsvAsApkg"), "info");
    }
  }

  async function stopAutoRefinement() {
    if (isStopping) return;
    isStopping = true;
    try { await invoke("refine_cancel"); }
    catch (error) {
      if (!disposed) { isStopping = false; snackbar.show(errorMessage(error), "error"); }
    } finally {
      if (!disposed && !autoRefining && !isSingleRefining) isStopping = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!active || e.defaultPrevented || e.isComposing || showOverwriteConfirm) return;
    const target = e.target instanceof Element ? e.target : null;
    if (target?.closest("input, textarea, select")) {
      if (e.key === "Escape" && document.activeElement instanceof HTMLElement) document.activeElement.blur();
      return;
    }
    if (target?.closest('[contenteditable]:not([contenteditable="false"]), [role="dialog"]')) return;
    const historyAction = historyShortcutAction(e, "refine");
    if (historyAction) {
      if (operationBusy) return;
      e.preventDefault();
      applyDocumentHistory(historyAction);
      return;
    }
    if (mode !== "manual" || selectedCardIndex === null || cards.length === 0) return;
    if (e.key === "ArrowDown" || e.key === "j") { e.preventDefault(); navigateCard(1); }
    else if (e.key === "ArrowUp" || e.key === "k") { e.preventDefault(); navigateCard(-1); }
    else if (e.key === "Enter") {
      e.preventDefault();
      const textarea = document.getElementById("card-notes") as HTMLTextAreaElement | null;
      if (textarea) { textarea.focus(); textarea.setSelectionRange(textarea.value.length, textarea.value.length); }
    }
  }

  function applyDocumentHistory(direction: "undo" | "redo") {
    if (operationBusy) return;
    if (direction === "undo") documentHistory.undo();
    else documentHistory.redo();
    historyRevision++;
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
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
      const path = (e.dataTransfer.files[0] as any).path;
      if (path && /\.(apkg|tsv)$/i.test(path)) await triggerLoadFile(path);
      else snackbar.show(t("refine.msg.unsupportedFormat"), "error");
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="h-full flex flex-col bg-gray-900 relative overflow-hidden text-gray-100"
  ondragover={handleDragOver}
  ondrop={handleDrop}
  ondragleave={handleDragLeave}
>
  {#if isDraggingOver}
    <div class="absolute inset-0 z-50 bg-rose-500/10 border-rose-400/80 text-rose-400 border-2 border-dashed rounded-2xl flex items-center justify-center pointer-events-none">
      <div class="text-center">
        <svg class="w-16 h-16 mx-auto mb-3 text-rose-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
        </svg>
        <p class="text-lg font-medium text-rose-300">{t("refine.dropFileHere")}</p>
        <p class="text-sm text-gray-400 mt-1">{t("refine.dropFileHint")}</p>
      </div>
    </div>
  {/if}

  <div class="flex flex-1 flex-col overflow-hidden p-6 min-h-0">
    <WorkflowWorkspace fillViewport>
      {#snippet files()}
        <RefineFilesPanel
          {filePath}
          totalCards={cards.length}
          {annotatedCount}
          {modifiedCount}
          disabled={operationBusy}
          highlightClass={requirements.classFor("files")}
          onBrowse={selectFile}
        />
      {/snippet}

      <div class="grid grid-cols-1 md:grid-cols-12 gap-6 items-stretch shrink-0 md:shrink md:flex-1 md:min-h-[280px]">
        <RefineCardListPanel
          {cards}
          filteredCount={filteredCards.length}
          {virtualVisibleCards}
          {totalVirtualHeight}
          {virtualTopOffset}
          {selectedCardIndex}
          bind:searchQuery
          canUndo={canUndoDocument}
          canRedo={canRedoDocument}
          {operationBusy}
          {cardIndices}
          {cardPlainTexts}
          {isCardRefining}
          bind:cardsContainerEl
          bind:cardListViewportHeight
          onScroll={handleCardListScroll}
          onHistory={applyDocumentHistory}
          onSelectCard={(index) => (selectedCardIndex = index)}
        />

        <div class="md:col-span-7 glass-card perf-scroll-card p-5 flex flex-col min-h-0 overflow-hidden">
          <div class="flex items-center justify-between mb-4 shrink-0">
            <h3 class="text-lg font-semibold flex items-center gap-2 text-amber-400">
              <svg class="w-5 h-5 text-amber-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h14a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
              </svg>
              {t("refine.title")}
            </h3>
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
                  {t("refine.mode.manual")}
                </button>
                <button
                  type="button"
                  aria-pressed={mode === "auto"}
                  onclick={() => setMode("auto")}
                  class="relative z-10 flex-1 py-1 px-2.5 text-center text-xs transition-colors duration-200 cursor-pointer {mode === 'auto' ? 'text-white font-bold' : 'text-gray-400 hover:text-white font-semibold'}"
                >
                  {t("refine.mode.auto")}
                </button>
              </div>
            {/if}
          </div>

          {#if mode === "manual"}
            <RefineManualPanel
              {selectedCard}
              {selectedCardIndex}
              {selectedFilteredIndex}
              filteredCount={filteredCards.length}
              plainText={selectedCard ? cardPlainTexts.get(selectedCard.id) : undefined}
              bind:notesValue={notesProxy.value}
              readonly={isSaving || isLoading || autoRefining || selectedCardIndex === null || !!(selectedCard && isCardRefining(selectedCard.id))}
              isGeneratingSelected={!!(selectedCard && isCardRefining(selectedCard.id))}
              {isSingleRefining}
              {autoRefining}
              {isStopping}
              generateDisabled={operationBusy || isValidatingLlm || !!llmError || selectedCardIndex === null}
              onStop={stopAutoRefinement}
              onGenerateSingle={refineSingleCardAI}
              onBlockedGenerate={handleBlockedGenerateSingle}
              onNavigate={navigateCard}
            />
          {:else}
            <RefineAutoPanel
              {useTiers}
              {tierCounts}
              {isValidatingLlm}
              llmErrorText={llmError ? llmErrorMessage(llmError) : null}
              {llmHighlightPulse}
              {operationBusy}
              bind:onlyUnannotated
              bind:useBatchMode
              {autoRefining}
              {isStopping}
              hasCards={cards.length > 0}
              {progressCurrent}
              {progressFailed}
              {progressTotal}
              {runSummary}
              {logs}
              onEditLlmSettings={() => onGoToSettings?.("llm", "default-refinement-prompt")}
              onStartAuto={startAutoRefinement}
              onBlockedStartAuto={handleBlockedStartAuto}
              onStopAuto={stopAutoRefinement}
            />
          {/if}
        </div>
      </div>
    </WorkflowWorkspace>
  </div>

  <RefineFooterBar
    {filePath}
    {fileName}
    {fileExtension}
    hasCards={cards.length > 0}
    {operationBusy}
    {isSaving}
    onOverwrite={overwriteOriginalFile}
    onSaveAs={saveNewFileWithExtension}
    onBlockedSave={handleBlockedSave}
  />

  <ConfirmDialog
    show={showOverwriteConfirm}
    title={t("refine.warning.unsavedChangesTitle")}
    message={t("refine.warning.unsavedChangesMsg")}
    confirmText={t("refine.warning.confirmLoad")}
    cancelText={t("common.cancel")}
    variant="warning"
    on:cancel={() => { showOverwriteConfirm = false; pendingPathToLoad = null; }}
    on:confirm={async () => {
      showOverwriteConfirm = false;
      if (pendingPathToLoad) { await loadFile(pendingPathToLoad); pendingPathToLoad = null; }
    }}
  />
</div>
