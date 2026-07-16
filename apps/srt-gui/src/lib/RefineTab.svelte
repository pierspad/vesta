<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { guardedOpen, guardedSave } from "./utils/dialogGuard";
  import { snackbar } from "./snackbarStore.svelte";
  import { onMount, onDestroy, untrack } from "svelte";
  import { locale, currentLanguage } from "./i18n";
  import { getFileName } from "./models";
  import { loadAndValidateApiKeys, type ApiKeyConfig } from "./apiKeys";
  import {
    loadTiers,
    tiersHaveUsableEntries,
    TIERS_UPDATED_EVENT,
    type Tier,
  } from "./translationTiers";
  import {
    buildTiersPayload,
    checkTiersAvailability,
    countTiersAndEndpoints,
    type TierEntryPayload,
    type TiersUnavailableReason,
  } from "./llmTiers";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import CodeEditor from "./CodeEditor.svelte";
  import { aiStore } from "./aiStore.svelte";
  import { loadRefinementPrompt } from "./refinementPrompt";
  import EmptyState from "./components/EmptyState.svelte";
  import FooterActions from "./components/FooterActions.svelte";

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
  let customPrompt = $state("");
  let onlyUnannotated = $state(true);
  let useBatchMode = $state(true);
  let isSingleRefining = $state(false);

  // States to track card-specific generation activity
  let singleRefiningCardIds = $state<string[]>([]);
  let autoRefineGroupCardIds = $state<string[]>([]);

  $effect(() => {
    aiStore.autoRefining = autoRefining;
    aiStore.isSingleRefining = isSingleRefining;
  });

  $effect(() => {
    if (aiStore.killSwitchActive && mode === "auto") {
      mode = "manual";
    }
  });

  const isCardRefining = (cardId: string) => {
    return singleRefiningCardIds.includes(cardId) || autoRefineGroupCardIds.includes(cardId);
  };

  // LLM tiers (stessa configurazione della traduzione: Settings → LLM)
  let tiers = $state<Tier[]>([]);
  let apiKeys = $state<ApiKeyConfig[]>([]);
  let useTiers = $derived(tiersHaveUsableEntries(tiers));
  let tierCounts = $derived(countTiersAndEndpoints(tiers));

  let filteredCards = $derived(
    cards.filter(
      (c) =>
        c.expression.toLowerCase().includes(searchQuery.toLowerCase()) ||
        c.meaning.toLowerCase().includes(searchQuery.toLowerCase()) ||
        c.notes.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  let selectedCard = $derived(
    selectedCardIndex !== null && cards[selectedCardIndex]
      ? cards[selectedCardIndex]
      : null
  );

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
    if (hasUnsavedChanges) {
      pendingPathToLoad = path;
      showOverwriteConfirm = true;
    } else {
      await loadFile(path);
    }
  }

  // Drag & drop state
  let isDraggingOver = $state(false);

  // Carica tiers + API key (stessa fonte di TranslateTab) e il prompt.
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
    switch (reason) {
      case "noneConfigured":
        return t("tiers.noneConfigured") || "No tiers configured";
      case "localOffline":
        return t("settings.llmConfigIncompleteDescLocalOffline") || "Local LLM server is offline";
      case "keyMissing":
        return t("settings.llmConfigIncompleteDescKey") || "Missing API key";
      case "incomplete":
        return t("settings.llmConfigIncompleteDescCustomEmpty") || "LLM configuration incomplete";
      default:
        return t("tiers.noneConfigured") || "No usable LLM endpoint";
    }
  }

  /// Payload per i comandi Tauri; `null` se nessun endpoint è utilizzabile.
  function refineTiersPayload(): TierEntryPayload[][] | null {
    if (!useTiers) return null;
    return buildTiersPayload(tiers, apiKeys);
  }

  $effect(() => {
    if (active) {
      untrack(() => {
        refreshLlmConfig();
      });
      let unlistenDragDropLocal: (() => void) | null = null;
      getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "over") {
          isDraggingOver = true;
        } else if (event.payload.type === "drop") {
          isDraggingOver = false;
          if (event.payload.paths && event.payload.paths.length > 0) {
            const path = event.payload.paths[0];
            if (path && (path.endsWith(".apkg") || path.endsWith(".tsv"))) {
              void triggerLoadFile(path);
            } else {
              snackbar.show(t("refine.msg.unsupportedFormat"), "error");
            }
          }
        } else if (event.payload.type === "leave") {
          isDraggingOver = false;
        }
      }).then((fn) => {
        unlistenDragDropLocal = fn;
      }).catch((e) => {
        console.warn("Failed to set up drag-drop listener in RefineTab:", e);
      });

      return () => {
        if (unlistenDragDropLocal) {
          unlistenDragDropLocal();
        }
      };
    }
  });

  onMount(() => {
    refreshLlmConfig();
    window.addEventListener(TIERS_UPDATED_EVENT, refreshLlmConfig);
    window.addEventListener("apikeys-updated", refreshLlmConfig);
  });

  onDestroy(() => {
    window.removeEventListener(TIERS_UPDATED_EVENT, refreshLlmConfig);
    window.removeEventListener("apikeys-updated", refreshLlmConfig);
  });

  // Load file
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
    isLoading = true;
    try {
      const res = await invoke<RefineCard[]>("refine_load_file", { path });
      cards = res.map((c) => ({ ...c, initialNotes: c.notes }));
      filePath = path;
      selectedCardIndex = res.length > 0 ? 0 : null;
      singleRefiningCardIds = [];
      autoRefineGroupCardIds = [];
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

  // Overwrite original loaded file
  async function overwriteOriginalFile() {
    if (cards.length === 0 || !filePath) return;
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

  // Save updates to a new file with specified extension
  async function saveNewFileWithExtension(ext: "apkg" | "tsv") {
    if (cards.length === 0) return;
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
          snackbar.show(t("refine.msg.saveSuccess"), "success");
          filePath = selected;
          cards = cards.map((c) => ({ ...c, initialNotes: c.notes }));
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


  // Refine single card using AI (pool a tier con failover)
  async function refineSingleCardAI() {
    if (selectedCardIndex === null || cards.length === 0) return;

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
      // L'engine si occupa di strip HTML e interpolazione del prompt.
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

      cards[selectedCardIndex].notes = response.trim();
      snackbar.show(t("refine.msg.generateSuccess"), "success");
    } catch (err: any) {
      snackbar.show(t("refine.msg.generateError", { error: err.toString() }), "error");
    } finally {
      singleRefiningCardIds = singleRefiningCardIds.filter((id) => id !== card.id);
      isSingleRefining = false;
    }
  }

  // Payload dell'evento Tauri `refine-progress` (serde tag "type").
  type RefineProgressPayload =
    | { type: "cardDone"; id: string; notes: string; done: number; total: number }
    | { type: "cardFailed"; id: string; error: string }
    | { type: "info"; message: string };

  interface RefineRunSummary {
    done: number;
    failed: number;
    poolExhausted: boolean;
    cancelled: boolean;
  }

  // Automatic refinement: il loop (batching, fallback, failover) vive nel
  // backend (`srt-refine::refine_cards_tiered`); qui solo eventi e stato UI.
  async function startAutoRefinement() {
    if (cards.length === 0) return;
    if (autoRefining) return;

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
    autoRefineGroupCardIds = cardsToProcess.map((c) => c.id);
    progressTotal = cardsToProcess.length;
    progressCurrent = 0;
    const endpointCount = tiersPayload.reduce((sum, tier) => sum + tier.length, 0);
    logs = [
      t("tiers.logActive", { tiers: tiersPayload.length, endpoints: endpointCount }),
      t("refine.log.cardsToProcess", { count: progressTotal }),
      t("refine.log.startAuto"),
    ];

    const unlisten = await listen<RefineProgressPayload>("refine-progress", (event) => {
      const p = event.payload;
      if (p.type === "cardDone") {
        const idx = cards.findIndex((c) => c.id === p.id);
        if (idx !== -1) {
          cards[idx].notes = p.notes;
          logs = [t("refine.log.success", { text: cards[idx].expression.substring(0, 30) }), ...logs];
        }
        progressCurrent = p.done;
      } else if (p.type === "cardFailed") {
        const idx = cards.findIndex((c) => c.id === p.id);
        const text = idx !== -1 ? cards[idx].expression.substring(0, 20) : p.id;
        logs = [t("refine.log.error", { text, error: p.error }), ...logs];
      } else {
        logs = [`[INFO] ${p.message}`, ...logs];
      }
    });

    try {
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
      });

      if (summary.cancelled) {
        logs = [t("refine.log.stopped"), ...logs];
      } else if (summary.poolExhausted) {
        snackbar.show(
          t("refine.msg.poolExhausted") || "All LLM tiers are exhausted (rate limit/quota)",
          "error",
        );
      } else {
        snackbar.show(
          t("refine.log.completed", { success: summary.done, total: progressTotal }),
          "success",
        );
      }
    } catch (err: any) {
      snackbar.show(t("refine.msg.generateError", { error: err.toString() }), "error");
    } finally {
      unlisten();
      autoRefining = false;
      autoRefineGroupCardIds = [];
    }
  }

  async function stopAutoRefinement() {
    try {
      await invoke("refine_cancel");
    } catch {
      /* run già terminato */
    }
  }

  // Keyboard navigation
  function handleKeyDown(e: KeyboardEvent) {
    if (selectedCardIndex === null || cards.length === 0) return;
    if (document.activeElement?.tagName === "TEXTAREA" || document.activeElement?.tagName === "INPUT") {
      // Allow escaping textarea focusing via escape key
      if (e.key === "Escape") {
        (document.activeElement as HTMLElement).blur();
      }
      return;
    }

    if (e.key === "ArrowDown" || e.key === "j") {
      e.preventDefault();
      if (selectedCardIndex < filteredCards.length - 1) {
        const nextCard = filteredCards[selectedCardIndex + 1];
        selectedCardIndex = cards.findIndex((c) => c.id === nextCard.id);
      }
    } else if (e.key === "ArrowUp" || e.key === "k") {
      e.preventDefault();
      if (selectedCardIndex > 0) {
        const prevCard = filteredCards[selectedCardIndex - 1];
        selectedCardIndex = cards.findIndex((c) => c.id === prevCard.id);
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      const textarea = document.getElementById("card-notes") as HTMLTextAreaElement | null;
      if (textarea) {
        textarea.focus();
        textarea.setSelectionRange(textarea.value.length, textarea.value.length);
      }
    }
  }

  // Drag & drop handlers
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
      const path = (file as any).path; // Tauri injected file path
      if (path && (path.endsWith(".apkg") || path.endsWith(".tsv"))) {
        await loadFile(path);
      } else {
        snackbar.show("Formato file non supportato. Trascina solo file .apkg o .tsv", "error");
      }
    }
  }

  // Direct generation link from FlashcardsTab
  if (typeof window !== "undefined") {
    (window as any).loadRefineFile = (path: string) => {
      void loadFile(path);
    };
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<!-- Main Container -->
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
  
  <!-- Content Area (padded) -->
  <div class="flex-1 flex flex-col min-h-0 p-6 overflow-hidden">


    <!-- Content Workspace -->
    <!-- Content Workspace -->
    <div class="flex-1 flex gap-5 overflow-hidden min-h-0">
      
      <!-- Left sidebar: Cards List -->
      <div class="w-[340px] flex flex-col bg-white/[0.02] border border-white/5 rounded-2xl overflow-hidden shrink-0">
        
        <!-- File Browse & Search bar -->
        <div class="p-3 border-b border-white/5 bg-white/[0.01] flex flex-col gap-2 shrink-0">
          <button
            onclick={selectFile}
            disabled={isLoading}
            class="w-full py-2 px-3 bg-indigo-600 hover:bg-indigo-500 text-white disabled:opacity-50 rounded-xl font-bold text-xs flex items-center justify-center gap-1.5 transition-colors cursor-pointer"
          >
            {#if isLoading}
              <svg class="animate-spin h-3.5 w-3.5 text-white" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              {t('refine.dropzone.loading')}
            {:else}
              <svg class="w-3.5 h-3.5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
              </svg>
              {t('refine.dropzone.browse')}
            {/if}
          </button>
          <div class="relative">
            <span class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-gray-500">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
            </span>
            <input
              type="text"
              bind:value={searchQuery}
              placeholder={t('refine.searchPlaceholder')}
              class="input-modern w-full pr-4 py-2 text-xs"
              style="padding-left: 2.75rem !important;"
            />
          </div>
        </div>

          <!-- Scrollable cards list -->
          <div class="flex-1 overflow-y-auto p-2 space-y-1.5 scrollbar-thin">
            {#each filteredCards as card, index}
              {@const isSelected = selectedCardIndex !== null && cards[selectedCardIndex]?.id === card.id}
              {@const globalIndex = cards.findIndex(c => c.id === card.id)}
              <button
                onclick={() => selectedCardIndex = globalIndex}
                class="w-full text-left p-3 rounded-xl transition-all border flex flex-col gap-1 cursor-pointer
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
                    {#if card.initialNotes && card.initialNotes.trim() !== ""}
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
          </div>
        </div>

        <!-- Right Column: Refine Tooling Workspace -->
        <div class="flex-1 flex flex-col overflow-hidden">

          {#snippet selectFileAction()}
            <button
              onclick={selectFile}
              class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl font-bold text-xs transition-colors cursor-pointer"
            >
              {t('refine.dropzone.browse')}
            </button>
          {/snippet}

          <!-- Segment control for Manual/Auto modes -->
          {#if !aiStore.killSwitchActive}
            <div class="flex items-center gap-1 bg-white/5 border border-white/5 p-1 rounded-xl w-fit mb-4 shrink-0">
              <button
                onclick={() => mode = "manual"}
                class="px-4 py-2 rounded-lg text-xs font-bold transition-all cursor-pointer
                  {mode === 'manual'
                    ? 'bg-indigo-600 text-white shadow-md'
                    : 'text-gray-400 hover:text-white hover:bg-white/5'}"
              >
                {t('refine.mode.manual')}
              </button>
              <button
                onclick={() => { mode = "auto"; refreshLlmConfig(); }}
                class="px-4 py-2 rounded-lg text-xs font-bold transition-all cursor-pointer
                  {mode === 'auto'
                    ? 'bg-indigo-600 text-white shadow-md'
                    : 'text-gray-400 hover:text-white hover:bg-white/5'}"
              >
                {t('refine.mode.auto')}
              </button>
            </div>
          {/if}

          <!-- Mode Containers -->
          {#if mode === "manual"}
            <!-- MANUAL MODE -->
            <div class="flex-1 flex flex-col bg-white/[0.02] border border-white/5 rounded-2xl overflow-hidden p-6 gap-5 min-h-0">
              {#if cards.length === 0}
                <EmptyState
                  title={t('refine.dropzone.title')}
                  description={t('refine.dropzone.desc')}
                  iconPath="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                  action={selectFileAction}
                />
              {:else}
              <!-- Cards content preview -->
              <div class="grid grid-cols-2 gap-4 shrink-0">
                <!-- Front/Expression -->
                <div class="bg-white/5 border border-white/10 rounded-xl p-4 flex flex-col gap-1.5 relative">
                  <span class="absolute top-2 right-3 text-[10px] font-bold text-gray-500 uppercase tracking-wider">{t('refine.card.front')}</span>
                  <div class="text-sm font-semibold text-gray-200 mt-2">
                    {@html selectedCard?.expression || "—"}
                  </div>
                </div>

                <!-- Back/Meaning -->
                <div class="bg-white/5 border border-white/10 rounded-xl p-4 flex flex-col gap-1.5 relative">
                  <span class="absolute top-2 right-3 text-[10px] font-bold text-gray-500 uppercase tracking-wider">{t('refine.card.back')}</span>
                  <div class="text-sm font-semibold text-gray-200 mt-2">
                    {@html selectedCard?.meaning || "—"}
                  </div>
                </div>
              </div>

              <!-- Notes editing textarea -->
              <div class="flex-1 flex flex-col min-h-0">
                <div class="flex justify-between items-center mb-2">
                  <label for="card-notes" class="block text-xs font-semibold text-gray-400">{t('refine.notesLabel')}</label>
                  {#if !aiStore.killSwitchActive}
                    <button
                      type="button"
                      onclick={refineSingleCardAI}
                      disabled={selectedCardIndex === null || (selectedCard && isCardRefining(selectedCard.id))}
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
                  readonly={selectedCardIndex === null || !!(selectedCard && isCardRefining(selectedCard.id))}
                  placeholder={selectedCardIndex === null ? t('refine.notesPlaceholderEmpty') : (!!(selectedCard && isCardRefining(selectedCard.id)) ? t('refine.notesPlaceholderGenerating') : t('refine.notesPlaceholder'))}
                  language="html"
                  heightClass="flex-1 min-h-[250px]"
                  textareaClass={selectedCardIndex === null || !!(selectedCard && isCardRefining(selectedCard.id)) ? 'opacity-70' : ''}
                />
              </div>

              <!-- Bottom navigation shortcuts -->
              <div class="flex justify-between items-center text-xs text-gray-400 border-t border-white/5 pt-4 shrink-0">
                <span class="flex items-center gap-1.5">
                  <kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">↑</kbd>/<kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">k</kbd> {t('refine.btn.prev')}
                  <span class="opacity-30 mx-1">•</span>
                  <kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">↓</kbd>/<kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">j</kbd> {t('refine.btn.next')}
                </span>
                
                <div class="flex items-center gap-2">
                  <button
                    onclick={() => {
                      if (selectedCardIndex !== null && selectedCardIndex > 0) selectedCardIndex--;
                    }}
                    disabled={selectedCardIndex === null || selectedCardIndex === 0}
                    class="bg-white/5 hover:bg-white/10 disabled:opacity-30 border border-white/10 rounded-lg px-3 py-1.5 font-semibold transition-colors cursor-pointer"
                  >
                    {t('refine.btn.prev')}
                  </button>
                  <button
                    onclick={() => {
                      if (selectedCardIndex !== null && selectedCardIndex < cards.length - 1) selectedCardIndex++;
                    }}
                    disabled={selectedCardIndex === null || selectedCardIndex === cards.length - 1}
                    class="bg-white/5 hover:bg-white/10 disabled:opacity-30 border border-white/10 rounded-lg px-3 py-1.5 font-semibold transition-colors cursor-pointer"
                  >
                    {t('refine.btn.next')}
                  </button>
                </div>
              </div>
              {/if}
            </div>
          {:else}
            <!-- AUTOMATIC MODE -->
            <div class="flex-1 flex flex-col bg-white/[0.02] border border-white/5 rounded-2xl p-6 gap-5 overflow-y-auto min-h-0">
              
              <!-- LLM Tiers status (clickable: apre Settings → LLM) -->
              <button
                type="button"
                onclick={() => onGoToSettings?.("llm", "default-refinement-prompt")}
                class="flex flex-col gap-2.5 bg-white/5 border border-white/10 hover:bg-white/10 hover:border-white/20 rounded-xl p-4 shrink-0 transition-all duration-200 cursor-pointer select-none text-left w-full"
                class:llm-requirement-pulse={llmHighlightPulse}
              >
                <!-- Line 1: tier summary -->
                <div class="flex items-center gap-2 w-full text-xs text-gray-400">
                  <svg class="w-4 h-4 text-indigo-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                  </svg>
                  <span class="font-semibold text-gray-300">{t("refine.llmTiersLabel")}</span>
                  {#if useTiers}
                    <span class="text-[10px] bg-indigo-500/10 border border-indigo-500/20 text-indigo-300 px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ml-1">
                      {t("refine.llmTiersSummary", { tiers: tierCounts.tiers, endpoints: tierCounts.endpoints })}
                    </span>
                  {:else}
                    <span class="text-[10px] bg-white/5 border border-white/10 text-gray-500 px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ml-1">
                      —
                    </span>
                  {/if}
                </div>

                <!-- Line 2: availability status -->
                <div class="flex items-center gap-2 w-full text-xs text-gray-400 flex-wrap">
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

              <!-- Options as premium toggleable card buttons -->
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4 shrink-0">
                <button
                  type="button"
                  onclick={() => onlyUnannotated = !onlyUnannotated}
                  class="flex flex-col items-start p-4 rounded-xl border text-left transition-all duration-200 cursor-pointer select-none
                    {onlyUnannotated
                      ? 'bg-rose-500/10 border-rose-500/50 text-white shadow-lg shadow-rose-950/20'
                      : 'bg-white/5 border-white/10 text-gray-400 hover:bg-white/10 hover:text-gray-200'}"
                >
                  <div class="flex items-center gap-2 mb-1.5">
                    <div class="w-6 h-6 rounded-lg flex items-center justify-center {onlyUnannotated ? 'bg-rose-500/20 text-rose-300' : 'bg-white/5 text-gray-500'}">
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                      </svg>
                    </div>
                    <span class="text-xs font-bold uppercase tracking-wider">{t('refine.options.onlyUnannotated')}</span>
                  </div>
                  <p class="text-[11px] text-gray-400 leading-normal">
                    {$currentLanguage === 'it' ? 'Elabora solo le flashcard che non hanno ancora annotazioni.' : 'Only process flashcards that do not have any notes yet.'}
                  </p>
                </button>

                <button
                  type="button"
                  onclick={() => useBatchMode = !useBatchMode}
                  class="flex flex-col items-start p-4 rounded-xl border text-left transition-all duration-200 cursor-pointer select-none
                    {useBatchMode
                      ? 'bg-rose-500/10 border-rose-500/50 text-white shadow-lg shadow-rose-950/20'
                      : 'bg-white/5 border-white/10 text-gray-400 hover:bg-white/10 hover:text-gray-200'}"
                >
                  <div class="flex items-center gap-2 mb-1.5">
                    <div class="w-6 h-6 rounded-lg flex items-center justify-center {useBatchMode ? 'bg-rose-500/20 text-rose-300' : 'bg-white/5 text-gray-500'}">
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                      </svg>
                    </div>
                    <span class="text-xs font-bold uppercase tracking-wider">{t('refine.options.batchMode')}</span>
                  </div>
                  <p class="text-[11px] text-gray-400 leading-normal">
                    {$currentLanguage === 'it' ? 'Invia più flashcard insieme per ridurre i tempi e i consumi dell\'API.' : 'Send multiple flashcards at once to speed up and reduce API usage.'}
                  </p>
                </button>
              </div>

              <!-- Start & Edit Buttons -->
              <div class="flex items-center gap-4 mt-2 shrink-0">
                <button
                  onclick={() => onGoToSettings?.("llm", "default-refinement-prompt")}
                  class="flex-1 rounded-xl bg-white/5 hover:bg-white/10 border border-white/10 text-sm font-bold text-gray-300 px-5 py-3 shadow-md transition-all cursor-pointer flex items-center justify-center gap-2 hover:scale-[1.02] active:scale-[0.98]"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h14a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                  </svg>
                  {t('refine.btn.editPrompt') || ($currentLanguage === 'it' ? 'Modifica Prompt' : 'Edit Prompt')}
                </button>

                {#if autoRefining}
                  <button
                    onclick={stopAutoRefinement}
                    class="flex-1 rounded-xl bg-red-600/80 hover:bg-red-500/80 border border-red-500/30 text-sm font-bold text-red-100 px-5 py-3 shadow-md transition-all cursor-pointer flex items-center justify-center gap-2"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                    {t('refine.btn.stop')}
                  </button>
                {:else}
                  <button
                    onclick={startAutoRefinement}
                    disabled={!!llmError || cards.length === 0}
                    class="flex-1 rounded-xl bg-rose-600/80 hover:bg-rose-500/80 border border-rose-500/30 disabled:bg-rose-600/40 text-sm font-bold text-rose-100 px-5 py-3 shadow-md transition-all cursor-pointer flex items-center justify-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] {(llmError || cards.length === 0) ? 'opacity-50 cursor-not-allowed' : ''}"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                    </svg>
                    {t('refine.btn.startAI') || ($currentLanguage === 'it' ? 'Avvia Rifinitura AI' : 'Start AI Refinement')}
                  </button>
                {/if}
              </div>

              <!-- Progress Bar -->
              {#if autoRefining || progressTotal > 0}
                <div class="bg-white/5 border border-white/10 rounded-xl p-4 shrink-0 mt-2">
                  <div class="flex justify-between text-xs text-gray-300 mb-2 font-semibold">
                    <span>{t('refine.progress.title')}</span>
                    <span>{progressCurrent} / {progressTotal} ({Math.round((progressCurrent / progressTotal) * 100)}%)</span>
                  </div>
                  <div class="w-full bg-white/10 h-2 rounded-full overflow-hidden">
                    <div
                      class="bg-gradient-to-r from-rose-500 to-pink-500 h-full rounded-full transition-all duration-300"
                      style="width: {(progressCurrent / progressTotal) * 100}%"
                    ></div>
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </div>

      </div>
  </div>

  <!-- Fixed Bottom Band with Action Buttons -->
  <FooterActions>
    {#snippet left()}
      <div class="flex items-center gap-4">
      <div class="text-xs text-gray-400 bg-white/5 border border-white/10 px-3 py-1.5 rounded-lg max-w-[300px] truncate" title={filePath || undefined}>
        <span class="font-bold text-gray-300">{t('refine.action.fileLabel')}</span> {fileName || "—"}
      </div>

      <div class="relative group">
        <button
          onclick={overwriteOriginalFile}
          disabled={isSaving || cards.length === 0}
          class="px-5 py-2.5 bg-rose-600 hover:bg-rose-500 disabled:bg-rose-600/40 text-white disabled:opacity-55 rounded-xl font-bold text-sm transition-all shadow-lg shadow-rose-950/30 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] cursor-pointer border border-rose-500/10"
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
          {t('refine.action.tooltipOverwrite') || ($currentLanguage === 'it' ? 'Sovrascrivi il file originale con le modifiche correnti' : 'Overwrite the original file with your current changes')}
        </div>
      </div>
      </div>
    {/snippet}
    {#snippet right()}
      <div class="flex items-center gap-4">
      <div class="relative group">
        <button
          onclick={() => saveNewFileWithExtension("apkg")}
          disabled={isSaving || cards.length === 0 || fileExtension === "TSV"}
          class="px-5 py-2.5 bg-emerald-600 hover:bg-emerald-500 disabled:bg-emerald-600/55 disabled:opacity-55 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed cursor-pointer"
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
            {t('refine.action.saveAsApkg') || ($currentLanguage === 'it' ? 'Salva come APKG' : 'Save as APKG')}
          {/if}
        </button>
        <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-emerald-500/30 bg-gray-950/95 p-3 text-center text-xs text-emerald-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {fileExtension === "TSV"
            ? ($currentLanguage === 'it' ? "Non è possibile salvare un TSV come APKG in questa scheda" : "It is not possible to save a TSV file as APKG in this tab")
            : (t('refine.action.tooltipSaveAsApkg') || ($currentLanguage === 'it' ? 'Salva il mazzo corrente come pacchetto Anki (.apkg)' : 'Save the current deck as an Anki package (.apkg)'))}
        </div>
      </div>

      <div class="relative group">
        <button
          onclick={() => saveNewFileWithExtension("tsv")}
          disabled={isSaving || cards.length === 0}
          class="px-5 py-2.5 bg-cyan-600 hover:bg-cyan-500 disabled:bg-cyan-600/55 disabled:opacity-55 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-cyan-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed cursor-pointer"
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
            {t('refine.action.saveAsTsv') || ($currentLanguage === 'it' ? 'Salva come TSV' : 'Save as TSV')}
          {/if}
        </button>
        <div class="pointer-events-none absolute bottom-full right-0 z-50 mb-3 rounded-xl border border-cyan-500/30 bg-gray-950/95 p-3 text-center text-xs text-cyan-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {t('refine.action.tooltipSaveAsTsv') || ($currentLanguage === 'it' ? 'Esporta il mazzo corrente come file di testo separato da tab (.tsv)' : 'Export the current deck as a tab-separated text file (.tsv)')}
        </div>
      </div>
      </div>
    {/snippet}
  </FooterActions>

  <ConfirmDialog
    show={showOverwriteConfirm}
    title={t('refine.warning.unsavedChangesTitle') || "Modifiche non salvate"}
    message={t('refine.warning.unsavedChangesMsg') || "Ci sono modifiche non salvate nel file corrente. Caricando un nuovo file, perderai i progressi non salvati. Vuoi procedere comunque?"}
    confirmText={t('refine.warning.confirmLoad') || "Procedi comunque"}
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
