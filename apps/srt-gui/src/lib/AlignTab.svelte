<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { guardedOpen, guardedSave } from './dialogGuard';
  import PathPreviewModal from './PathPreviewModal.svelte';
  import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
  import { t } from './i18n';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import { languages } from './models';
  import InfoButton from './InfoButton.svelte';

  interface Subtitle {
    id: number;
    start: string;
    end: string;
    text: string;
  }

  let targetPath = $state("");
  let sourcePath = $state("");
  
  let targetSubs: Subtitle[] = $state([]);
  let sourceSubs: Subtitle[] = $state([]);

  let currentPage = $state(0);
  const ITEMS_PER_PAGE_OPTIONS = [5, 10, 15, 20] as const;
  let itemsPerPageIndex = $state(2); // default 15
  let itemsPerPage = $derived(ITEMS_PER_PAGE_OPTIONS[itemsPerPageIndex]);

  function setItemsPerPageIndex(nextIndex: number) {
    const firstVisibleIdx = currentPage * itemsPerPage;
    const clampedIndex = Math.max(0, Math.min(nextIndex, ITEMS_PER_PAGE_OPTIONS.length - 1));
    const newPerPage = ITEMS_PER_PAGE_OPTIONS[clampedIndex];
    const newTotalPages = Math.ceil(Math.max(targetSubs.length, sourceSubs.length) / newPerPage);
    itemsPerPageIndex = clampedIndex;
    currentPage = Math.min(
      Math.floor(firstVisibleIdx / newPerPage),
      Math.max(0, newTotalPages - 1),
    );
  }

  function cycleItemsPerPage() {
    setItemsPerPageIndex((itemsPerPageIndex + 1) % ITEMS_PER_PAGE_OPTIONS.length);
  }

  let error = $state("");
  let success = $state("");
  let helpSection = $state<string | null>(null);

  interface ActivityLogEntry {
    id: number;
    timestamp: string;
    message: string;
    level: 'info' | 'success' | 'warning' | 'error';
  }
  let activityLogs = $state<ActivityLogEntry[]>([]);
  let activityLogId = 0;

  function addActivityLog(message: string, level: ActivityLogEntry['level'] = 'info') {
    const timestamp = new Date().toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
    activityLogs = [...activityLogs, { id: ++activityLogId, timestamp, message, level }].slice(-80);
  }

  function clearActivityLogs() {
    activityLogs = [];
    activityLogId = 0;
  }

  // Expanded path modal
  let expandedPathField = $state<string | null>(null);

  // ─── Undo History ──────────────────────────────────────────────────────────
  const MAX_UNDO = 50;
  let undoStack = $state<string[]>([]);
  let undoDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  function pushUndo() {
    // Serialize current sourceSubs state
    const snapshot = JSON.stringify(sourceSubs.map(s => ({ id: s.id, start: s.start, end: s.end, text: s.text })));
    // Don't push if identical to last snapshot
    if (undoStack.length > 0 && undoStack[undoStack.length - 1] === snapshot) return;
    undoStack = [...undoStack.slice(-(MAX_UNDO - 1)), snapshot];
  }

  function scheduleUndo() {
    // Push undo snapshot on first keystroke, debounce subsequent ones
    if (undoDebounceTimer === null) {
      pushUndo();
    } else {
      clearTimeout(undoDebounceTimer);
    }
    undoDebounceTimer = setTimeout(() => {
      undoDebounceTimer = null;
    }, 500);
  }

  function performUndo() {
    if (undoStack.length === 0) return;
    const snapshot = undoStack[undoStack.length - 1];
    undoStack = undoStack.slice(0, -1);
    try {
      const parsed = JSON.parse(snapshot) as Subtitle[];
      sourceSubs = parsed;
      addActivityLog('Undo applied', 'warning');
    } catch {}
  }

  function handleKeydown(e: KeyboardEvent) {
    if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') return;

    // Ctrl+O → open target (1st SRT)
    if (e.key === 'o' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault();
      selectTarget();
      return;
    }
    // Ctrl+Z → undo
    if (e.key === 'z' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault();
      performUndo();
      return;
    }
    // Ctrl+Shift+S → swap files
    if (e.key === 'S' && (e.ctrlKey || e.metaKey) && e.shiftKey) {
      e.preventDefault();
      swapFiles();
      return;
    }
    // Ctrl+Shift+P → cycle items per page
    if (e.key === 'P' && (e.ctrlKey || e.metaKey) && e.shiftKey) {
      e.preventDefault();
      cycleItemsPerPage();
      return;
    }
    // Ctrl+S → save
    if (e.key === 's' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault();
      saveSource();
      return;
    }
    // Tab → next page, Shift+Tab → prev page (only when not in a textarea)
    if (e.key === 'Tab' && !(document.activeElement?.tagName === 'TEXTAREA')) {
      e.preventDefault();
      if (e.shiftKey) {
        prevPage();
      } else {
        nextPage();
      }
      return;
    }
  }

  // ─── Language / Flag Detection ─────────────────────────────────────────────
  const knownLangCodes = new Set(languages.map(l => l.code.toLowerCase()));

  function inferLanguageFromPath(filePath: string): string | null {
    const filename = filePath.split("/").pop()?.toLowerCase() || "";
    const base = filename.replace(/\.[^/.]+$/, "");
    // Split on separators: . - _
    const tokens = base.split(/[.\-_]+/).filter(Boolean);
    // Check from the end for a known language code
    for (let i = tokens.length - 1; i >= 0; i--) {
      if (knownLangCodes.has(tokens[i])) {
        // Return the original-case code from the languages list
        const lang = languages.find(l => l.code.toLowerCase() === tokens[i]);
        if (lang) return lang.code;
      }
    }
    return null;
  }

  function getFlagForPath(path: string): string {
    const code = inferLanguageFromPath(path);
    if (!code) return "";
    const lang = languages.find(l => l.code === code);
    return lang?.flag || "";
  }

  let targetFlag = $derived(targetPath ? getFlagForPath(targetPath) : "");
  let sourceFlag = $derived(sourcePath ? getFlagForPath(sourcePath) : "");

  // ─── Jump to Empty Subtitle ────────────────────────────────────────────────
  function findNextEmptyPage(direction: 'forward' | 'backward'): number | null {
    if (sourceSubs.length === 0) return null;
    const startIdx = direction === 'forward' 
      ? (currentPage + 1) * itemsPerPage 
      : currentPage * itemsPerPage - 1;
    
    if (direction === 'forward') {
      for (let i = startIdx; i < sourceSubs.length; i++) {
        if (!sourceSubs[i].text || sourceSubs[i].text.trim() === '') {
          return Math.floor(i / itemsPerPage);
        }
      }
    } else {
      for (let i = Math.min(startIdx, sourceSubs.length - 1); i >= 0; i--) {
        if (!sourceSubs[i].text || sourceSubs[i].text.trim() === '') {
          return Math.floor(i / itemsPerPage);
        }
      }
    }
    return null;
  }

  let hasEmptyForward = $derived(findNextEmptyPage('forward') !== null);
  let hasEmptyBackward = $derived(findNextEmptyPage('backward') !== null);

  function jumpToNextEmpty() {
    const page = findNextEmptyPage('forward');
    if (page !== null) currentPage = page;
  }

  function jumpToPrevEmpty() {
    const page = findNextEmptyPage('backward');
    if (page !== null) currentPage = page;
  }

  // DnD listener unsubs
  let unlistenFileDrop: () => void;

  onMount(async () => {
    window.addEventListener('keydown', handleKeydown);
    unlistenFileDrop = await listen<{ paths: string[] }>('tauri://file-drop', (event) => {
      const paths = event.payload.paths;
      if (paths && paths.length > 0) {
        handleDroppedFiles(paths);
      }
    });
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    if (unlistenFileDrop) unlistenFileDrop();
    if (undoDebounceTimer) clearTimeout(undoDebounceTimer);
  });

  function handleDroppedFiles(paths: string[]) {
    // Assign to source or target based on what's empty, or just overwrite first
    for (const p of paths) {
      if (!p.toLowerCase().endsWith('.srt')) continue;
      
      if (!targetPath) {
        loadTarget(p);
      } else if (!sourcePath) {
        loadSource(p);
      } else {
        // Overwrite source if both full
        loadSource(p);
      }
    }
  }

  function parseSrt(content: string): Subtitle[] {
    const blocks = content.trim().replace(/\r\n/g, '\n').split(/\n\s*\n/);
    return blocks.map(block => {
      const lines = block.split('\n');
      const id = parseInt(lines[0], 10) || 0;
      const timeLine = lines[1] || '';
      const times = timeLine.split(' --> ');
      const text = lines.slice(2).join('\n');
      return { id, start: times[0] || '00:00:00,000', end: times[1] || '00:00:00,000', text };
    });
  }

  function serializeSrt(subs: Subtitle[]): string {
    return subs.map(s => `${s.id}\n${s.start} --> ${s.end}\n${s.text}`).join('\n\n') + '\n';
  }

  function normalizeAlignments() {
    // Collect all unique IDs across both arrays
    const targetMap = new Map(targetSubs.map(s => [s.id, s]));
    const sourceMap = new Map(sourceSubs.map(s => [s.id, s]));
    
    // Create a Set of all IDs, sort numerically
    const allIds = Array.from(new Set([...targetMap.keys(), ...sourceMap.keys()])).sort((a, b) => a - b);
    
    // Rebuild the arrays based on the unique IDs, injecting padded items where missing
    targetSubs = allIds.map(id => {
      if (targetMap.has(id)) return targetMap.get(id)!;
      // If missing in target, inject a dummy object
      const s = sourceMap.get(id);
      return { id, start: s?.start || '00:00:00,000', end: s?.end || '00:00:00,000', text: '' };
    });

    sourceSubs = allIds.map(id => {
      if (sourceMap.has(id)) return sourceMap.get(id)!;
      // If missing in source, inject an empty editable item
      const t = targetMap.get(id);
      return { id, start: t?.start || '00:00:00,000', end: t?.end || '00:00:00,000', text: '' };
    });
  }

  async function loadTarget(path: string) {
    try {
      const content = await readTextFile(path);
      targetSubs = parseSrt(content);
      targetPath = path;
      normalizeAlignments();
      error = "";
      addActivityLog(`Target loaded: ${getFileName(path)} (${targetSubs.length} subtitles)`, 'success');
    } catch (e) {
      error = `Error loading target: ${e}`;
      addActivityLog(`Target load failed: ${e}`, 'error');
    }
  }

  async function tryAutoSelectSourceForTarget(targetSrtPath: string) {
    if (sourcePath) return;
    try {
      const suggested = await invoke<string | null>("sync_suggest_companion_subtitle_for_srt", {
        srtPath: targetSrtPath,
      });
      if (!suggested || suggested === targetSrtPath) return;
      await loadSource(suggested);
    } catch {
      // Best-effort suggestion only.
    }
  }

  async function tryAutoSelectTargetForSource(sourceSrtPath: string) {
    if (targetPath) return;
    try {
      const suggested = await invoke<string | null>("sync_suggest_companion_subtitle_for_srt", {
        srtPath: sourceSrtPath,
      });
      if (!suggested || suggested === sourceSrtPath) return;
      await loadTarget(suggested);
    } catch {
      // Best-effort suggestion only.
    }
  }

  async function loadSource(path: string) {
    try {
      const content = await readTextFile(path);
      sourceSubs = parseSrt(content);
      sourcePath = path;
      normalizeAlignments();
      undoStack = []; // Reset undo on new file load
      error = "";
      addActivityLog(`Source loaded: ${getFileName(path)} (${sourceSubs.length} subtitles)`, 'success');
    } catch (e) {
      error = `Error loading source: ${e}`;
      addActivityLog(`Source load failed: ${e}`, 'error');
    }
  }

  async function selectTarget() {
    const selected = await guardedOpen({
      filters: [{ name: 'Subtitles', extensions: ['srt'] }]
    });
    if (selected && !Array.isArray(selected)) {
      await loadTarget(selected);
      await tryAutoSelectSourceForTarget(selected);
    }
  }

  async function selectSource() {
    const selected = await guardedOpen({
      filters: [{ name: 'Subtitles', extensions: ['srt'] }]
    });
    if (selected && !Array.isArray(selected)) {
      await loadSource(selected);
      await tryAutoSelectTargetForSource(selected);
    }
  }

  function swapFiles() {
    const tempPath = targetPath;
    targetPath = sourcePath;
    sourcePath = tempPath;

    const tempSubs = targetSubs;
    targetSubs = sourceSubs;
    sourceSubs = tempSubs;
    
    normalizeAlignments();
    addActivityLog('Target and source swapped', 'info');
  }

  async function saveSource() {
    try {
      const defaultPath = sourcePath.replace('.srt', '_aligned.srt');
      const savePath = await guardedSave({
        defaultPath,
        filters: [{ name: 'Subtitles', extensions: ['srt'] }]
      });

      if (savePath) {
        const content = serializeSrt(sourceSubs);
        await writeTextFile(savePath, content);
        success = `File saved to ${savePath}`;
        addActivityLog(`Aligned file saved: ${getFileName(savePath)}`, 'success');
        setTimeout(() => success = "", 3000);
      }
    } catch (e) {
      error = `Error saving file: ${e}`;
      addActivityLog(`Save failed: ${e}`, 'error');
    }
  }

  let totalPages = $derived(Math.ceil(Math.max(targetSubs.length, sourceSubs.length) / itemsPerPage));

  $effect(() => {
    if (currentPage >= totalPages) {
      currentPage = Math.max(0, totalPages - 1);
    }
  });
  
  // Create padded arrays for the current page so we can iterate side-by-side
  let currentPageItems = $derived(Array.from({ length: itemsPerPage }, (_, i) => {
    const index = currentPage * itemsPerPage + i;
    return {
      index,
      target: targetSubs[index] || null,
      source: sourceSubs[index] || null
    };
  }).filter(item => item.target !== null || item.source !== null));

  function prevPage() {
    if (currentPage > 0) currentPage--;
  }

  function nextPage() {
    if (currentPage < totalPages - 1) currentPage++;
  }

  function jumpStart() {
    currentPage = 0;
  }

  function jumpEnd() {
    if (totalPages > 0) currentPage = totalPages - 1;
  }


  function getFileName(path: string): string {
    return path.split('/').pop() || path;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class="h-full flex flex-col p-6 overflow-y-auto relative text-gray-200 bg-gradient-to-br from-gray-900 via-gray-900 to-gray-950"
  onkeydown={handleKeydown}
>
  <div class="min-h-full flex flex-col gap-4">
  <div class="glass-card p-6 shrink-0">
    <div class="mb-6 flex items-start justify-between shrink-0 gap-3">
      <div>
        <h2 class="text-lg font-semibold text-teal-300 flex items-center gap-2">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16m-7 6h7"/>
          </svg>
          {t("nav.revision")}
        </h2>
        <p class="text-sm text-gray-500 mt-0.5">{t("nav.revision.desc")}</p>
      </div>
      <InfoButton
        class="text-gray-500 hover:text-teal-300 transition-colors p-1"
        title={t("align.helpTitle")}
        onclick={() => (helpSection = "overview")}
      />
    </div>

    <!-- Error/Success -->
    {#if error}
      <div class="mb-4 p-3 bg-red-500/10 border border-red-500/50 rounded-xl text-red-400 flex items-center shrink-0">
        {error}
        <button class="ml-auto" onclick={() => error = ""}>✕</button>
      </div>
    {/if}
    {#if success}
      <div class="mb-4 p-3 bg-green-500/10 border border-green-500/50 rounded-xl text-green-400 flex items-center shrink-0">
        {success}
        <button class="ml-auto" onclick={() => success = ""}>✕</button>
      </div>
    {/if}

    <!-- File Selection Area -->
    <div class="grid grid-cols-1 md:grid-cols-[1fr_auto_1fr] items-center gap-4 shrink-0 relative min-w-0">
    
    <!-- Target File -->
    <div class="flex flex-col gap-2 relative z-10 min-w-0">
      <div class="text-sm font-semibold text-gray-300 flex items-center gap-2">
        {#if targetFlag}<span class="text-lg">{targetFlag}</span>{/if}
        Original / base SRT
      </div>
      <div class="flex gap-2 min-w-0">
        <button onclick={selectTarget} class="btn-primary whitespace-nowrap px-4 py-2 shrink-0 flex items-center gap-2 shadow-lg shadow-teal-500/15">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7a2 2 0 012-2h5l2 2h7a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z" /></svg>
          Open SRT
        </button>
        <button 
          type="button"
          onclick={() => expandedPathField = "target"}
          class="input-modern flex-1 text-sm text-left cursor-pointer hover:bg-white/10 transition-colors truncate min-w-0"
          style="direction: rtl; text-align: left;"
          title={targetPath || "Drag & drop SRT here..."}
        >
          <span
            class={targetPath ? "text-white" : "text-gray-500"}
            style="unicode-bidi: plaintext;"
          >
            {targetPath || "Drag & drop SRT here..."}
          </span>
        </button>
      </div>
      {#if targetSubs.length > 0}
        <div class="text-xs text-gray-400">{targetSubs.length} subtitles loaded</div>
      {/if}
    </div>

    <!-- Swap Button -->
    <div class="flex items-center justify-center relative z-20 md:px-2 shrink-0 mt-4 md:mt-0">
      <button 
        onclick={swapFiles}
        disabled={!targetPath || !sourcePath}
        class="p-2.5 rounded-full border transition-colors group {(!targetPath || !sourcePath) ? 'bg-gray-900/40 text-gray-600 border-gray-800 cursor-not-allowed' : 'bg-gray-900/70 hover:bg-teal-500/20 text-gray-400 hover:text-teal-300 border-gray-700 hover:border-teal-500/50'}"
        title="Swap file order"
      >
        <svg class="w-6 h-6 {!targetPath || !sourcePath ? '' : 'group-hover:rotate-180'} transition-transform duration-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
        </svg>
      </button>
    </div>

    <!-- Source File -->
    <div class="flex flex-col gap-2 relative z-10 min-w-0">
      <div class="text-sm font-semibold text-gray-300 flex items-center gap-2">
        {#if sourceFlag}<span class="text-lg">{sourceFlag}</span>{/if}
        Translation / review SRT
      </div>
      <div class="flex gap-2 min-w-0">
        <button 
          onclick={selectSource} 
          disabled={!targetPath}
          class="whitespace-nowrap px-4 py-2 shrink-0 flex items-center gap-2 rounded-lg border transition-all font-semibold {!targetPath ? 'bg-white/5 text-gray-600 border-transparent cursor-not-allowed' : 'bg-indigo-500/20 text-indigo-200 border-indigo-500/30 hover:bg-indigo-500/30 hover:border-indigo-400/50 shadow-lg shadow-indigo-500/10'}"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7a2 2 0 012-2h5l2 2h7a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z" /></svg>
          Open SRT
        </button>
        <button 
          type="button"
          disabled={!targetPath}
          onclick={() => targetPath && (expandedPathField = "source")}
          class="input-modern flex-1 text-sm text-left truncate min-w-0 {!targetPath ? 'opacity-50 cursor-not-allowed bg-transparent' : 'cursor-pointer hover:bg-white/10 transition-colors'}"
          style="direction: rtl; text-align: left;"
          title={sourcePath || "Drag & drop SRT here..."}
        >
          <span
            class={sourcePath ? "text-white" : "text-gray-500"}
            style="unicode-bidi: plaintext;"
          >
            {sourcePath || "Drag & drop SRT here..."}
          </span>
        </button>
      </div>
      {#if sourceSubs.length > 0}
        <div class="text-xs text-gray-400">{sourceSubs.length} subtitles loaded</div>
      {/if}
    </div>
    </div>

  </div>

    <!-- Editor Area -->
    <div class="flex-1 flex flex-col min-w-0 glass-card p-6 overflow-hidden">
      {#if targetSubs.length === 0 && sourceSubs.length === 0}
        <div class="flex-1 flex flex-col items-center justify-center text-gray-500 pb-10">
        <svg class="w-20 h-20 mb-6 opacity-20 text-teal-500 bg-teal-500/5 p-4 rounded-full" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
        <p class="text-lg font-medium text-gray-400">Load some SRT files to start aligning.</p>
        <p class="text-sm mt-2 text-gray-600">You can drag and drop files anywhere on this window.</p>
        </div>
      {:else}
        <!-- Pagination Top -->
        <div class="flex flex-col xl:flex-row xl:items-center xl:justify-between mb-4 gap-3 shrink-0">
        <div class="flex items-center gap-2 flex-wrap">
          <div class="min-w-[10.5rem] text-sm text-gray-400 font-medium bg-gray-800/80 px-3 py-1.5 rounded-md tabular-nums">
             Page <span class="inline-block min-w-[4ch] text-center text-white mx-1">{currentPage + 1}</span> of <span class="inline-block min-w-[4ch] text-center text-white mx-1">{totalPages || 1}</span>
          </div>
          <div class="w-px h-6 bg-gray-700 mx-1"></div>
          <div class="flex items-center gap-1 bg-gray-800/50 px-1 py-1 rounded-md border border-gray-700">
            {#each ITEMS_PER_PAGE_OPTIONS as option, optionIndex}
              <button
                onclick={() => setItemsPerPageIndex(optionIndex)}
                class="min-w-8 rounded px-2 py-0.5 text-xs font-medium transition-colors {itemsPerPage === option
                  ? 'bg-teal-500/20 text-teal-200'
                  : 'text-gray-400 hover:bg-white/10 hover:text-gray-200'}"
                title="{option} subtitles per page"
              >
                {option}
              </button>
            {/each}
          </div>
        </div>
        <div class="flex gap-1.5 flex-wrap xl:justify-end">
          <button onclick={jumpStart} disabled={currentPage === 0} class="btn-secondary px-3 py-1.5 disabled:opacity-50 flex items-center" title="Go to Start">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7" /></svg>
          </button>
          <!-- Jump to Prev Empty (orange glow) -->
          <button onclick={jumpToPrevEmpty} disabled={!hasEmptyBackward} class="btn-secondary px-3 py-1.5 disabled:opacity-30 flex items-center empty-jump-btn" title="Jump to previous empty subtitle">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7M22 19l-7-7 7-7" /></svg>
          </button>
          <button onclick={prevPage} disabled={currentPage === 0} class="btn-secondary px-4 py-1.5 disabled:opacity-50 flex items-center">
            <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" /></svg>
            Prev
          </button>
          <button onclick={nextPage} disabled={currentPage >= totalPages - 1} class="btn-secondary px-4 py-1.5 disabled:opacity-50 flex items-center">
            Next
            <svg class="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" /></svg>
          </button>
          <!-- Jump to Next Empty (orange glow) -->
          <button onclick={jumpToNextEmpty} disabled={!hasEmptyForward} class="btn-secondary px-3 py-1.5 disabled:opacity-30 flex items-center empty-jump-btn" title="Jump to next empty subtitle">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M2 5l7 7-7 7" /></svg>
          </button>
          <button onclick={jumpEnd} disabled={currentPage >= totalPages - 1} class="btn-secondary px-3 py-1.5 disabled:opacity-50 flex items-center" title="Go to End">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" /></svg>
          </button>
          <div class="w-px h-6 bg-gray-700 mx-1"></div>
          <button onclick={saveSource} class="btn-primary px-6 py-1.5 flex items-center gap-2 shadow-lg shadow-teal-500/20" disabled={sourceSubs.length === 0}>
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" /></svg>
            {#if sourceFlag}<span class="text-base">{sourceFlag}</span>{/if}
            Save Result
          </button>
        </div>
        </div>

        <!-- Content Grid -->
        <div class="pr-3 pl-1 space-y-6 custom-scrollbar pb-4 min-w-0 overflow-y-auto flex-1">
        {#each currentPageItems as item (item.index)}
          {@const isMissingPair =
            (!!item.source && (!item.source.text || item.source.text.trim() === '')) ||
            (!!item.target && (!item.target.text || item.target.text.trim() === ''))}
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 group min-w-0 rounded-xl p-1 transition-all {isMissingPair ? 'missing-pair-row' : ''}">
            
            <!-- Target Side (Readonly) -->
            <div class="flex flex-col h-full rounded-lg border overflow-hidden relative bg-gray-900/40 min-w-0 {isMissingPair ? 'border-orange-500/50' : 'border-gray-800/70'}">
              {#if item.target}
                <!-- Header part -->
                <div class="flex justify-between items-center text-xs text-gray-500 bg-gray-900/80 px-3 py-2 border-b border-gray-800/50 font-mono tracking-wider truncate">
                  <span class="bg-gray-800 flex-shrink-0 px-2 py-0.5 rounded text-gray-400 max-w-full">#{item.target.id}</span>
                  <span class="flex items-center gap-1.5 md:gap-2 truncate ml-2">
                    <span class="text-teal-400/50 truncate min-w-0">{item.target.start}</span>
                    <svg class="w-3 h-3 mx-0.5 text-gray-600 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/></svg>
                    <span class="text-emerald-400/50 truncate min-w-0">{item.target.end}</span>
                  </span>
                </div>
                <!-- Content part -->
                <textarea 
                  class="flex-1 w-full bg-transparent p-3 text-sm text-gray-300 resize-none min-h-[90px] focus:outline-none min-w-0"
                  readonly
                  value={item.target.text}
                ></textarea>
              {:else}
                <div class="h-full min-h-[125px] flex flex-col items-center justify-center text-gray-600 text-sm">
                  <svg class="w-6 h-6 mb-2 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" /></svg>
                  <span>No Subtitle</span>
                </div>
              {/if}
            </div>

            <!-- Source Side (Editable) -->
            <div class="flex flex-col h-full rounded-lg border bg-indigo-950/10 focus-within:border-indigo-500/50 transition-colors overflow-hidden relative min-w-0 {isMissingPair ? 'border-orange-500/60' : 'border-indigo-500/20'}">
              {#if item.source}
                 <!-- Header part -->
                 <div class="flex justify-between items-center text-xs text-indigo-400/70 bg-indigo-950/40 px-3 py-2 border-b border-indigo-500/20 font-mono tracking-wider truncate">
                  <span class="bg-indigo-900/50 flex-shrink-0 text-indigo-300 px-2 py-0.5 rounded max-w-full">#{item.source.id}</span>
                  <span class="flex items-center gap-1.5 md:gap-2 truncate ml-2">
                    <span class="text-blue-400/70 truncate min-w-0">{item.source.start}</span>
                    <svg class="w-3 h-3 mx-0.5 text-indigo-500/50 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/></svg>
                    <span class="text-indigo-400/70 truncate min-w-0">{item.source.end}</span>
                  </span>
                </div>
                <!-- Content part: Svelte 5 runes allow easy bind:value on sourceSubs array -->
                <textarea 
                  class="flex-1 w-full bg-transparent p-3 text-[15px] leading-relaxed text-indigo-100 resize-none min-h-[90px] focus:outline-none placeholder-indigo-900/50 min-w-0"
                  bind:value={sourceSubs[item.index].text}
                  oninput={scheduleUndo}
                  placeholder="Type subtitle here..."
                ></textarea>
              {:else}
                <div class="h-full min-h-[125px] flex flex-col items-center justify-center text-indigo-900/40 text-sm">
                  <svg class="w-6 h-6 mb-2 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" /></svg>
                  <span>No Subtitle</span>
                </div>
              {/if}
            </div>
            
          </div>
        {/each}
        </div>
      {/if}
    </div>
  </div>

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "target" ? "1st SRT Path" : "2nd SRT Path"}
    value={expandedPathField === "target" ? targetPath || "—" : sourcePath || "—"}
    onclose={() => (expandedPathField = null)}
  />

  {#if helpSection}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-6"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={() => (helpSection = null)}
      onkeydown={(e) => {
        if (e.key === "Escape") helpSection = null;
      }}
    >
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="bg-gray-900 border border-gray-700 rounded-xl w-full max-w-2xl p-5 animate-fade-in"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
      >
        <div class="flex items-center justify-between mb-3">
          <h3 class="text-sm font-semibold text-gray-300">{t("align.helpTitle")}</h3>
          <button
            onclick={() => (helpSection = null)}
            class="text-gray-400 hover:text-white text-lg leading-none"
          >✕</button>
        </div>
        <div class="text-sm text-gray-200 leading-relaxed prose prose-invert prose-sm max-w-none">
          {@html t("align.helpContent")}
        </div>
        <div class="mt-4 flex justify-end">
          <button
            onclick={() => (helpSection = null)}
            class="btn-primary py-1.5 px-4 text-xs"
          >OK</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .missing-pair-row {
    border: 1px solid rgba(249, 115, 22, 0.45);
    box-shadow: 0 0 0 1px rgba(249, 115, 22, 0.15), 0 0 18px rgba(249, 115, 22, 0.12);
    background: linear-gradient(90deg, rgba(249, 115, 22, 0.07), rgba(249, 115, 22, 0.02));
  }

  .empty-jump-btn:not(:disabled) {
    color: #f97316;
    border-color: rgba(249, 115, 22, 0.4);
    filter: drop-shadow(0 0 6px rgba(249, 115, 22, 0.5));
    animation: empty-glow 2s ease-in-out infinite alternate;
  }
  .empty-jump-btn:not(:disabled):hover {
    background-color: rgba(249, 115, 22, 0.15);
    border-color: rgba(249, 115, 22, 0.6);
    filter: drop-shadow(0 0 10px rgba(249, 115, 22, 0.7));
  }
  @keyframes empty-glow {
    0% { filter: drop-shadow(0 0 4px rgba(249, 115, 22, 0.3)); }
    100% { filter: drop-shadow(0 0 8px rgba(249, 115, 22, 0.6)); }
  }
</style>
