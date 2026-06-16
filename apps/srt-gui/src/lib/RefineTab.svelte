<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { guardedOpen, guardedSave } from "./utils/dialogGuard";
  import { snackbar } from "./snackbarStore.svelte";
  import { onMount } from "svelte";
  import { locale } from "./i18n";
  import { getFileName } from "./models";

  interface Props {
    active?: boolean;
    onGoToSettings?: (section: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightId?: string) => void;
  }

  let { active = true, onGoToSettings }: Props = $props();

  let t = $derived($locale);

  interface RefineCard {
    id: string;
    expression: string;
    meaning: string;
    notes: string;
  }

  interface RefineLlmConfig {
    api_type: string;
    api_key: string;
    api_url: string;
    model: string;
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

  // LLM Config state
  let llmConfig = $state<RefineLlmConfig>({
    api_type: "",
    api_key: "",
    api_url: "",
    model: "",
  });

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

  // Drag & drop state
  let isDraggingOver = $state(false);

  function loadStoredValue(key: string, fallback = ""): string {
    try {
      return localStorage.getItem(key) || fallback;
    } catch {
      return fallback;
    }
  }

  // Load LLM configuration from localStorage
  function refreshLlmConfig() {
    const provider = loadStoredValue("vesta-default-llm-provider", "local");
    const model = loadStoredValue("vesta-default-llm-model", "");
    const customProviderId = loadStoredValue("vesta-default-llm-custom-provider", "");
    const localUrl = loadStoredValue("vesta-local-server-url", "http://localhost:11434/v1");
    
    // Load default prompt
    const defaultPrompt = loadStoredValue(
      "vesta-default-refinement-prompt",
      "Spiega le parole desuete e più astruse della frase fornendo traduzione, esempio d'uso ed etimologia."
    );
    if (!customPrompt) {
      customPrompt = defaultPrompt;
    }

    let apiKey = "";
    let apiUrl = "";
    let activeModel = model;

    try {
      const keysRaw = loadStoredValue("srt-tools-api-keys", "[]");
      interface ApiKeyItem { id: string; apiType: string; apiKey: string; apiUrl?: string; model?: string; }
      const keys: ApiKeyItem[] = JSON.parse(keysRaw);

      if (provider === "google") {
        const gkey = keys.find((k) => k.apiType === "google");
        if (gkey) apiKey = gkey.apiKey;
        apiUrl = "https://generativelanguage.googleapis.com/v1beta";
        if (!activeModel) activeModel = "gemini-2.0-flash";
      } else if (provider === "groq") {
        const gkey = keys.find((k) => k.apiType === "groq");
        if (gkey) apiKey = gkey.apiKey;
        apiUrl = "https://api.groq.com/openai/v1";
        if (!activeModel) activeModel = "llama-3.3-70b-versatile";
      } else if (provider === "local") {
        apiKey = "";
        apiUrl = localUrl;
        if (!activeModel) activeModel = "llama3.2";
      } else if (provider === "custom") {
        const ckey = keys.find((k) => k.id === customProviderId);
        if (ckey) {
          apiKey = ckey.apiKey;
          apiUrl = ckey.apiUrl || "";
          if (!activeModel) activeModel = ckey.model || "";
        }
      }
    } catch (err) {
      console.error("Failed to parse API keys:", err);
    }

    llmConfig = {
      api_type: provider,
      api_key: apiKey,
      api_url: apiUrl,
      model: activeModel,
    };
  }

  onMount(() => {
    refreshLlmConfig();
    window.addEventListener("vesta-llm-default-updated", refreshLlmConfig);
    return () => {
      window.removeEventListener("vesta-llm-default-updated", refreshLlmConfig);
    };
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
        await loadFile(selected);
      }
    } catch (err: any) {
      snackbar.show(err.toString(), "error");
    }
  }

  async function loadFile(path: string) {
    isLoading = true;
    try {
      const res = await invoke<RefineCard[]>("refine_load_file", { path });
      cards = res;
      filePath = path;
      selectedCardIndex = res.length > 0 ? 0 : null;
      snackbar.show(
        `File caricato con successo: ${res.length} flashcard trovate.`,
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
        snackbar.show("File originale sovrascritto con successo!", "success");
      } else {
        snackbar.show("Impossibile sovrascrivere il file.", "error");
      }
    } catch (err: any) {
      snackbar.show(err.toString(), "error");
    } finally {
      isSaving = false;
    }
  }

  // Save updates to a new file
  async function saveNewFile() {
    if (cards.length === 0) return;
    try {
      const ext = filePath.split(".").pop()?.toLowerCase() || "apkg";
      const defaultName = fileName
        ? fileName.replace(`.${ext}`, `_refined.${ext}`)
        : `refined_deck.${ext}`;

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
          snackbar.show("Modifiche salvate con successo!", "success");
          filePath = selected;
        } else {
          snackbar.show("Impossibile salvare il file.", "error");
        }
      }
    } catch (err: any) {
      snackbar.show(err.toString(), "error");
    } finally {
      isSaving = false;
    }
  }

  // Refine single card using AI
  async function refineSingleCardAI() {
    if (selectedCardIndex === null || cards.length === 0) return;
    
    // Verify LLM Configuration
    if (llmConfig.api_type !== "local" && !llmConfig.api_key) {
      snackbar.show(
        "Manca la chiave API per il provider selezionato. Configurala nelle impostazioni.",
        "error"
      );
      if (onGoToSettings) {
        onGoToSettings("llm");
      }
      return;
    }

    const card = cards[selectedCardIndex];
    isSingleRefining = true;
    try {
      const response = await invoke<string>("refine_card_llm_with_config", {
        card: {
          id: card.id,
          expression: card.expression.replace(/<[^>]*>/g, ""), // strip HTML
          meaning: card.meaning.replace(/<[^>]*>/g, ""), // strip HTML
          notes: card.notes,
        },
        prompt: customPrompt,
        config: llmConfig,
      });

      cards[selectedCardIndex].notes = response.trim();
      snackbar.show("Nota generata con successo!", "success");
    } catch (err: any) {
      snackbar.show(`Errore durante la generazione: ${err.toString()}`, "error");
    } finally {
      isSingleRefining = false;
    }
  }

  // Automatic refinement logic
  async function startAutoRefinement() {
    if (cards.length === 0) return;
    if (autoRefining) return;

    // Verify LLM Configuration
    if (llmConfig.api_type !== "local" && !llmConfig.api_key) {
      snackbar.show(
        "Manca la chiave API per il provider selezionato. Configurala nelle impostazioni.",
        "error"
      );
      if (onGoToSettings) {
        onGoToSettings("llm");
      }
      return;
    }

    const cardsToProcess = onlyUnannotated
      ? cards.filter((c) => !c.notes || c.notes.trim() === "")
      : cards;

    if (cardsToProcess.length === 0) {
      snackbar.show("Nessuna flashcard da elaborare.", "info");
      return;
    }

    autoRefining = true;
    progressTotal = cardsToProcess.length;
    progressCurrent = 0;
    logs = ["Avvio della rifinitura automatica...", `Flashcard da elaborare: ${progressTotal}`];

    if (useBatchMode) {
      const batchSize = 5;
      for (let i = 0; i < cardsToProcess.length; i += batchSize) {
        if (!autoRefining) {
          logs = ["Elaborazione annullata dall'utente.", ...logs];
          break;
        }

        const batchCards = cardsToProcess.slice(i, i + batchSize);
        logs = [`[AI] Generazione note per batch di ${batchCards.length} flashcard...`, ...logs];

        try {
          const promptInstructions = `Sei un assistente AI specializzato nell'arricchimento e nella rifinitura di flashcard per l'apprendimento delle lingue.
Ti verrà fornito un elenco di flashcard in formato JSON.
Il tuo compito è generare note dettagliate per CIASCUNA flashcard seguendo SCRUPOLOSAMENTE questa istruzione:
"${customPrompt}"

---
LISTA DI FLASHCARD DA ELABORARE (formato JSON):
${JSON.stringify(batchCards.map(c => ({
  id: c.id,
  expression: c.expression.replace(/<[^>]*>/g, ""),
  meaning: c.meaning.replace(/<[^>]*>/g, "")
})), null, 2)}
---

Rispondi ESCLUSIVAMENTE con un oggetto JSON valido strutturato esattamente come il seguente esempio, senza includere commenti o spiegazioni aggiuntive al di fuori del JSON. Non racchiudere la risposta in blocchi di codice markdown (no \`\`\`json ... \`\`\`), restituisci solo il testo JSON.

Esempio di formato di risposta atteso:
{
  "results": [
    {
      "id": "id_da_lista",
      "notes": "spiegazione/note generate..."
    }
  ]
}
`;

          const response = await invoke<string>("refine_card_llm_with_config", {
            card: { id: "", expression: "", meaning: "", notes: "" },
            prompt: promptInstructions,
            config: llmConfig,
          });

          let cleaned = response.trim();
          if (cleaned.startsWith("```")) {
            cleaned = cleaned.replace(/^```[a-zA-Z]*\s*/, "").replace(/\s*```$/, "");
          }

          const parsed = JSON.parse(cleaned);
          if (parsed && Array.isArray(parsed.results)) {
            for (const res of parsed.results) {
              const cardId = res.id;
              const notesVal = res.notes || "";
              const origIdx = cards.findIndex(c => c.id === cardId);
              if (origIdx !== -1) {
                cards[origIdx].notes = notesVal.trim();
                progressCurrent++;
                logs = [`[OK] Note aggiunte per: "${cards[origIdx].expression.substring(0, 30)}..."`, ...logs];
              }
            }
          } else {
            throw new Error("Formato di risposta non valido (manca il campo 'results')");
          }
        } catch (err: any) {
          logs = [`[INFO] Errore batch (${err.message || err.toString()}), passaggio a elaborazione singola per questo blocco...`, ...logs];
          
          // Fallback to single card refinement for this batch
          for (const card of batchCards) {
            if (!autoRefining) break;
            const origIndex = cards.findIndex((c) => c.id === card.id);
            if (origIndex === -1) continue;

            logs = [`[AI-Fallback] Generazione note per: "${card.expression.substring(0, 30)}..."`, ...logs];

            try {
              const response = await invoke<string>("refine_card_llm_with_config", {
                card: {
                  id: card.id,
                  expression: card.expression.replace(/<[^>]*>/g, ""),
                  meaning: card.meaning.replace(/<[^>]*>/g, ""),
                  notes: card.notes,
                },
                prompt: customPrompt,
                config: llmConfig,
              });

              cards[origIndex].notes = response.trim();
              progressCurrent++;
              logs = [`[OK-Fallback] Note aggiunte per: "${card.expression.substring(0, 30)}..."`, ...logs];
            } catch (singleErr: any) {
              logs = [`[ERRORE-Fallback] Flashcard ${card.expression.substring(0, 20)}: ${singleErr.toString()}`, ...logs];
            }
          }
        }
      }
    } else {
      for (const card of cardsToProcess) {
        if (!autoRefining) {
          logs = ["Elaborazione annullata dall'utente.", ...logs];
          break;
        }

        const origIndex = cards.findIndex((c) => c.id === card.id);
        if (origIndex === -1) continue;

        logs = [`[AI] Generazione note per: "${card.expression.substring(0, 30)}..."`, ...logs];

        try {
          const response = await invoke<string>("refine_card_llm_with_config", {
            card: {
              id: card.id,
              expression: card.expression.replace(/<[^>]*>/g, ""),
              meaning: card.meaning.replace(/<[^>]*>/g, ""),
              notes: card.notes,
            },
            prompt: customPrompt,
            config: llmConfig,
          });

          cards[origIndex].notes = response.trim();
          progressCurrent++;
          logs = [`[OK] Note aggiunte per: "${card.expression.substring(0, 30)}..."`, ...logs];
        } catch (err: any) {
          logs = [`[ERRORE] Flashcard ${card.expression.substring(0, 20)}: ${err.toString()}`, ...logs];
        }
      }
    }

    autoRefining = false;
    snackbar.show(`Elaborazione automatica completata: ${progressCurrent}/${progressTotal} flashcard elaborate.`, "success");
  }

  function stopAutoRefinement() {
    autoRefining = false;
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

  function handleDragLeave() {
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
<div class="h-full flex flex-col bg-gray-900 relative overflow-hidden text-gray-100">
  
  <!-- Content Area (padded) -->
  <div class="flex-1 flex flex-col min-h-0 p-6 overflow-hidden">
    <!-- Header Bar -->
    <div class="flex items-center justify-between shrink-0 border-b border-white/5 pb-4 mb-4">
      <div>
        <h2 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
          <svg class="w-6 h-6 text-rose-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 8l1.5-1.5M19 4l1.5 1.5M16 5l1.5-1.5" />
          </svg>
          {t("nav.refine") || "Rifinisci Flashcard"}
        </h2>
        <p class="text-xs text-gray-400 mt-1">
          Rifinisci le tue note flashcard a mano o arricchiscile in automatico usando l'intelligenza artificiale.
        </p>
      </div>

      <!-- File indicator in header -->
      {#if cards.length > 0}
        <div class="text-xs text-gray-400 bg-white/5 border border-white/10 px-3 py-1.5 rounded-lg max-w-[300px] truncate" title={filePath}>
          <span class="font-bold text-gray-300">File:</span> {fileName}
        </div>
      {/if}
    </div>

    <!-- Content Workspace -->
    {#if cards.length === 0}
      <!-- Drop Zone / Initial State -->
      <div
        class="flex-1 flex flex-col items-center justify-center border-2 border-dashed rounded-2xl transition-all p-12 text-center select-none h-full
          {isDraggingOver ? 'border-rose-500 bg-rose-500/5' : 'border-white/10 bg-white/[0.02] hover:border-white/20'}"
        role="region"
        aria-label="Zona di caricamento file"
        ondragover={handleDragOver}
        ondragleave={handleDragLeave}
        ondrop={handleDrop}
      >
        <div class="w-16 h-16 rounded-2xl bg-white/5 border border-white/10 flex items-center justify-center text-gray-400 mb-6 shadow-xl">
          <svg class="w-8 h-8 text-rose-500 animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
          </svg>
        </div>
        <h3 class="text-lg font-bold text-white mb-2">Trascina qui il tuo file di flashcard</h3>
        <p class="text-sm text-gray-400 max-w-md mb-6 leading-relaxed">
          Supporta pacchetti Anki (.apkg) o file tabulati (.tsv) esportati da Vesta o altri strumenti.
        </p>
        
        <button
          onclick={selectFile}
          disabled={isLoading}
          class="inline-flex items-center gap-2 rounded-lg bg-rose-600 hover:bg-rose-500 text-sm font-bold text-white px-6 py-3.5 shadow-lg shadow-rose-600/20 transition-all cursor-pointer"
        >
          {#if isLoading}
            <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            Estrazione flashcard...
          {:else}
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            Sfoglia file flashcard
          {/if}
        </button>
      </div>
    {:else}
      <!-- Active Workspace -->
      <div class="flex-1 flex gap-5 overflow-hidden min-h-0">
        
        <!-- Left sidebar: Cards List -->
        <div class="w-[340px] flex flex-col bg-white/[0.02] border border-white/5 rounded-2xl overflow-hidden shrink-0">
          
          <!-- Search bar -->
          <div class="p-3 border-b border-white/5 bg-white/[0.01]">
            <div class="relative">
              <span class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-gray-500">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                </svg>
              </span>
              <input
                type="text"
                bind:value={searchQuery}
                placeholder="Cerca tra le flashcard..."
                class="input-modern w-full pl-9 pr-4 py-2 text-xs"
              />
            </div>
            <div class="text-[10px] text-gray-500 mt-2 flex justify-between">
              <span>Trovate: {filteredCards.length}</span>
              <span>Totali: {cards.length}</span>
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
                  {#if card.notes && card.notes.trim() !== ""}
                    <span class="bg-rose-500/20 text-rose-300 px-1.5 py-0.5 rounded-full border border-rose-500/20">Annotata</span>
                  {/if}
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
          
          <!-- Segment control for Manual/Auto modes -->
          <div class="flex items-center gap-1 bg-white/5 border border-white/5 p-1 rounded-xl w-fit mb-4 shrink-0">
            <button
              onclick={() => mode = "manual"}
              class="px-4 py-2 rounded-lg text-xs font-bold transition-all cursor-pointer
                {mode === 'manual'
                  ? 'bg-rose-600 text-white shadow-md'
                  : 'text-gray-400 hover:text-white hover:bg-white/5'}"
            >
              A Mano
            </button>
            <button
              onclick={() => { mode = "auto"; refreshLlmConfig(); }}
              class="px-4 py-2 rounded-lg text-xs font-bold transition-all cursor-pointer
                {mode === 'auto'
                  ? 'bg-rose-600 text-white shadow-md'
                  : 'text-gray-400 hover:text-white hover:bg-white/5'}"
            >
              In Automatico (AI)
            </button>
          </div>

          <!-- Mode Containers -->
          {#if mode === "manual"}
            <!-- MANUAL MODE -->
            {#if selectedCard}
              <div class="flex-1 flex flex-col bg-white/[0.02] border border-white/5 rounded-2xl overflow-hidden p-6 gap-5 min-h-0">
                
                <!-- Cards content preview -->
                <div class="grid grid-cols-2 gap-4 shrink-0">
                  <!-- Front/Expression -->
                  <div class="bg-white/5 border border-white/10 rounded-xl p-4 flex flex-col gap-1.5 relative">
                    <span class="absolute top-2 right-3 text-[10px] font-bold text-gray-500 uppercase tracking-wider">Fronte (Expression)</span>
                    <div class="text-sm font-semibold text-gray-200 mt-2">
                      {@html selectedCard.expression}
                    </div>
                  </div>

                  <!-- Back/Meaning -->
                  <div class="bg-white/5 border border-white/10 rounded-xl p-4 flex flex-col gap-1.5 relative">
                    <span class="absolute top-2 right-3 text-[10px] font-bold text-gray-500 uppercase tracking-wider">Retro (Meaning)</span>
                    <div class="text-sm font-semibold text-gray-200 mt-2">
                      {@html selectedCard.meaning}
                    </div>
                  </div>
                </div>

                <!-- Notes editing textarea -->
                <div class="flex-1 flex flex-col min-h-0">
                  <div class="flex justify-between items-center mb-2">
                    <label for="card-notes" class="block text-xs font-semibold text-gray-400">Note (Notes)</label>
                    <button
                      type="button"
                      onclick={refineSingleCardAI}
                      disabled={isSingleRefining}
                      class="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-indigo-600/20 hover:bg-indigo-600/35 border border-indigo-500/30 hover:border-indigo-500/50 text-indigo-300 text-xs font-bold transition-all duration-200 cursor-pointer disabled:opacity-50"
                    >
                      {#if isSingleRefining}
                        <svg class="animate-spin h-3.5 w-3.5 text-indigo-300" fill="none" viewBox="0 0 24 24">
                          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                        </svg>
                        Generazione...
                      {:else}
                        <svg class="w-3.5 h-3.5 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                        </svg>
                        Genera con IA (Prompt)
                      {/if}
                    </button>
                  </div>
                  <textarea
                    id="card-notes"
                    bind:value={cards[selectedCardIndex!].notes}
                    placeholder="Scrivi qui informazioni utili, traduzioni, note grammaticali ed etimologia..."
                    class="input-modern w-full flex-1 resize-none text-sm p-4 font-sans focus:border-rose-500/50 focus:ring-1 focus:ring-rose-500/20 leading-relaxed"
                  ></textarea>
                </div>

                <!-- Bottom navigation shortcuts -->
                <div class="flex justify-between items-center text-xs text-gray-400 border-t border-white/5 pt-4 shrink-0">
                  <span class="flex items-center gap-1.5">
                    <kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">↑</kbd>/<kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">k</kbd> Precedente
                    <span class="opacity-30 mx-1">•</span>
                    <kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">↓</kbd>/<kbd class="px-1.5 py-0.5 rounded bg-white/10 text-[10px] font-mono">j</kbd> Successivo
                  </span>
                  
                  <div class="flex items-center gap-2">
                    <button
                      onclick={() => {
                        if (selectedCardIndex! > 0) selectedCardIndex!--;
                      }}
                      disabled={selectedCardIndex === 0}
                      class="bg-white/5 hover:bg-white/10 disabled:opacity-30 border border-white/10 rounded-lg px-3 py-1.5 font-semibold transition-colors cursor-pointer"
                    >
                      Precedente
                    </button>
                    <button
                      onclick={() => {
                        if (selectedCardIndex! < cards.length - 1) selectedCardIndex!++;
                      }}
                      disabled={selectedCardIndex === cards.length - 1}
                      class="bg-white/5 hover:bg-white/10 disabled:opacity-30 border border-white/10 rounded-lg px-3 py-1.5 font-semibold transition-colors cursor-pointer"
                    >
                      Successivo
                    </button>
                  </div>
                </div>
              </div>
            {:else}
              <div class="flex-1 flex items-center justify-center bg-white/[0.02] border border-white/5 rounded-2xl p-6 text-gray-500 text-sm">
                Nessuna flashcard selezionata.
              </div>
            {/if}
          {:else}
            <!-- AUTOMATIC MODE -->
            <div class="flex-1 flex flex-col bg-white/[0.02] border border-white/5 rounded-2xl p-6 gap-5 overflow-y-auto min-h-0">
              
              <!-- LLM Status bar -->
              <div class="flex items-center justify-between bg-white/5 border border-white/10 rounded-xl p-4 shrink-0">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 rounded-lg bg-indigo-500/20 text-indigo-300 flex items-center justify-center">
                    <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                    </svg>
                  </div>
                  <div>
                    <span class="block text-[10px] text-gray-400 font-bold uppercase tracking-wider leading-none">Motore LLM in uso</span>
                    <span class="text-xs text-gray-100 font-semibold mt-1 block">
                      {llmConfig.api_type.toUpperCase()} • <span class="font-mono text-gray-300">{llmConfig.model || "Configurazione mancante"}</span>
                    </span>
                  </div>
                </div>
                <button
                  onclick={() => onGoToSettings?.("llm")}
                  class="text-xs text-indigo-400 hover:text-indigo-300 font-semibold cursor-pointer"
                >
                  Cambia modello
                </button>
              </div>

              <!-- Custom Prompt editor -->
              <div class="flex flex-col shrink-0">
                <div class="flex justify-between items-center mb-2">
                  <label for="auto-prompt" class="block text-xs font-semibold text-gray-400">Prompt di rifinitura per l'LLM</label>
                  <span class="text-[10px] text-gray-500">Supporta: `{"{{expression}}"}` (Fronte) e `{"{{meaning}}"}` (Retro)</span>
                </div>
                <textarea
                  id="auto-prompt"
                  bind:value={customPrompt}
                  rows="3"
                  placeholder="Spiega le parole più difficili fornendo etimologia ed esempi..."
                  class="input-modern w-full text-sm font-sans resize-none p-3 focus:border-rose-500/50"
                ></textarea>
              </div>

              <!-- Execution Options & Start -->
              <div class="flex items-center justify-between border-t border-white/5 pt-4 shrink-0">
                <div class="flex flex-col md:flex-row md:items-center gap-4">
                  <label class="flex items-center gap-2 text-xs text-gray-300 cursor-pointer select-none">
                    <input
                      type="checkbox"
                      bind:checked={onlyUnannotated}
                      class="rounded border-white/10 bg-white/5 text-rose-500 focus:ring-rose-500/20"
                    />
                    Processa solo flashcard senza note
                  </label>
                  <label class="flex items-center gap-2 text-xs text-gray-300 cursor-pointer select-none">
                    <input
                      type="checkbox"
                      bind:checked={useBatchMode}
                      class="rounded border-white/10 bg-white/5 text-rose-500 focus:ring-rose-500/20"
                    />
                    Elaborazione in batch (più veloce ed economico)
                  </label>
                </div>

                {#if autoRefining}
                  <button
                    onclick={stopAutoRefinement}
                    class="rounded-lg bg-red-600 hover:bg-red-500 text-xs font-bold text-white px-5 py-2.5 shadow-lg shadow-red-600/20 transition-colors cursor-pointer"
                  >
                    Interrompi
                  </button>
                {:else}
                  <button
                    onclick={startAutoRefinement}
                    class="rounded-lg bg-rose-600 hover:bg-rose-500 text-xs font-bold text-white px-5 py-2.5 shadow-lg shadow-rose-600/20 transition-all cursor-pointer"
                  >
                    Avvia Rifinitura AI
                  </button>
                {/if}
              </div>

              <!-- Progress Bar -->
              {#if autoRefining || progressTotal > 0}
                <div class="bg-white/5 border border-white/10 rounded-xl p-4 shrink-0">
                  <div class="flex justify-between text-xs text-gray-300 mb-2 font-semibold">
                    <span>Elaborazione automatica</span>
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

              <!-- Log Console -->
              <div class="flex-1 flex flex-col min-h-[150px]">
                <span class="block text-xs font-semibold text-gray-400 mb-2">Log elaborazione</span>
                <div class="flex-1 bg-black/40 border border-white/5 rounded-xl p-4 font-mono text-[10px] text-gray-300 overflow-y-auto space-y-1">
                  {#each logs as log}
                    <div class="leading-relaxed border-b border-white/[0.02] pb-1">
                      {log}
                    </div>
                  {/each}
                  {#if logs.length === 0}
                    <div class="text-gray-500 italic text-center p-4">I log verranno visualizzati qui all'avvio...</div>
                  {/if}
                </div>
              </div>
            </div>
          {/if}
        </div>

      </div>
    {/if}
  </div>

  <!-- Fixed Bottom Band with Action Buttons -->
  {#if cards.length > 0}
    <div class="h-[92px] border-t border-white/10 bg-gray-900 flex items-center justify-center gap-4 px-6 shrink-0 z-40">
      <button
        onclick={overwriteOriginalFile}
        disabled={isSaving}
        class="px-5 py-2.5 bg-rose-600 hover:bg-rose-500 disabled:bg-rose-600/55 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-rose-900/30 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:opacity-55 cursor-pointer"
      >
        {#if isSaving}
          <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          Salvataggio...
        {:else}
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
          </svg>
          Sovrascrivi {fileExtension}
        {/if}
      </button>

      <button
        onclick={saveNewFile}
        disabled={isSaving}
        class="px-5 py-2.5 bg-white/5 hover:bg-white/10 text-gray-300 rounded-xl font-bold text-sm border border-white/10 flex items-center gap-2 transition-all hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m-9 1V4a2 2 0 012-2h6l2 2h6a2 2 0 012 2v7a2 2 0 01-2 2H5a2 2 0 01-2-2z" />
        </svg>
        Salva nuovo {fileExtension}
      </button>

      <div class="w-px h-8 bg-white/10 mx-2"></div>

      <button
        onclick={() => { filePath = ""; cards = []; selectedCardIndex = null; }}
        class="px-5 py-2.5 bg-white/5 hover:bg-white/10 border border-white/10 text-gray-300 hover:text-white rounded-xl font-bold text-sm transition-colors cursor-pointer flex items-center gap-2"
      >
        Chiudi
      </button>
    </div>
  {/if}

</div>
