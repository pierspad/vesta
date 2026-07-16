<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { writeTextFile } from '@tauri-apps/plugin-fs';
  import { onDestroy, onMount } from 'svelte';
  import { guardedOpen, guardedSave } from './utils/dialogGuard';
  import { locale } from './i18n';
  import { getFileName, inferLanguageFromPath, getFlagForPath } from './models';
  import { languages } from './languages';
  import PathPreviewModal from './PathPreviewModal.svelte';
  import { snackbar } from './snackbarStore.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import PathPickerField from './PathPickerField.svelte';
  import EmptyState from './components/EmptyState.svelte';
  import FooterActions from './components/FooterActions.svelte';

  let { active = false } = $props<{ active?: boolean }>();

  let t = $derived($locale);

  interface Subtitle {
    id: number;
    start: string;
    end: string;
    text: string;
    originalStart?: string;
    originalEnd?: string;
  }

  let targetPath = $state("");
  let sourcePath = $state("");
  
  let targetSubs: Subtitle[] = $state([]);
  let sourceSubs: Subtitle[] = $state([]);

  let isDraggingOver = $state(false);
  let showOverwriteConfirm = $state(false);
  let pendingDroppedPaths = $state<string[]>([]);

  let hasUnsavedChanges = $state(false);
  let showUnsavedWarning = $state(false);
  let pendingBrowseAction = $state<(() => void) | null>(null);

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

  $effect(() => {
    if (error) {
      snackbar.show(error, "error", 4000);
      error = "";
    }
  });

  $effect(() => {
    if (success) {
      snackbar.show(success, "success", 3000);
      success = "";
    }
  });

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

  // Expanded path field
  let expandedPathField = $state<string | null>(null);

  // ─── Undo History ──────────────────────────────────────────────────────────
  const MAX_UNDO = 50;
  let undoStack = $state<string[]>([]);
  let undoDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  function pushUndo() {
    // Serialize current sourceSubs state
    const snapshot = JSON.stringify(sourceSubs.map(s => ({
      id: s.id,
      start: s.start,
      end: s.end,
      text: s.text,
      originalStart: s.originalStart,
      originalEnd: s.originalEnd
    })));
    // Don't push if identical to last snapshot
    if (undoStack.length > 0 && undoStack[undoStack.length - 1] === snapshot) return;
    undoStack = [...undoStack.slice(-(MAX_UNDO - 1)), snapshot];
  }

  function scheduleUndo() {
    hasUnsavedChanges = true;
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

  function handleTextareaWheel(e: WheelEvent) {
    const textarea = e.currentTarget as HTMLTextAreaElement;
    const direction = e.deltaY;
    if (direction === 0) return;

    // Se la textarea stessa può scorrere nella direzione dello scorrimento richiesto,
    // lasciamo che l'evento avvenga nativamente senza bloccarlo.
    const textareaCanScroll = textarea.scrollHeight > textarea.clientHeight;
    if (textareaCanScroll) {
      if (direction > 0 && textarea.scrollTop + textarea.clientHeight < textarea.scrollHeight - 1) {
        return;
      }
      if (direction < 0 && textarea.scrollTop > 1) {
        return;
      }
    }

    // Altrimenti, cerchiamo l'antenato scorrevole più vicino che ha spazio libero per scorrere nella direzione corretta.
    const findScrollableAncestor = (el: HTMLElement, dir: number): HTMLElement | null => {
      let parent = el.parentElement;
      while (parent) {
        const style = window.getComputedStyle(parent);
        const overflowY = style.overflowY;
        const isScrollableStyle = overflowY === 'auto' || overflowY === 'scroll' || 
                                  parent.classList.contains('overflow-y-auto') || 
                                  parent.classList.contains('overflow-auto');
        
        if (isScrollableStyle && parent.scrollHeight > parent.clientHeight) {
          if (dir > 0 && parent.scrollTop + parent.clientHeight < parent.scrollHeight - 1) {
            return parent;
          }
          if (dir < 0 && parent.scrollTop > 1) {
            return parent;
          }
        }
        parent = parent.parentElement;
      }
      return null;
    };

    const scrollContainer = findScrollableAncestor(textarea, direction);
    if (scrollContainer) {
      // Normalizzazione del delta a seconda della modalità (fondamentale in ambienti WebKit/Tauri su Linux/Windows)
      let scrollAmount = e.deltaY;
      if (e.deltaMode === 1) { // Modalità a righe
        scrollAmount *= 40;
      } else if (e.deltaMode === 2) { // Modalità a pagine
        scrollAmount *= 800;
      }

      scrollContainer.scrollBy({
        top: scrollAmount,
        behavior: 'auto'
      });
      e.preventDefault();
    }
  }



  let targetFlag = $derived(targetPath ? getFlagForPath(targetPath) : "");
  let sourceFlag = $derived(sourcePath ? getFlagForPath(sourcePath) : "");

  // ─── Jump to Empty Subtitle ────────────────────────────────────────────────
  function findNextEmptyPage(direction: 'forward' | 'backward'): number | null {
    if (!targetPath || !sourcePath || sourceSubs.length === 0) return null;
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

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);

    let activeListener = true;
    let unlistenDD: (() => void) | null = null;

    getCurrentWebview().onDragDropEvent((event) => {
      if (!active) return;
      if (event.payload.type === "over") {
        isDraggingOver = true;
      } else if (event.payload.type === "drop") {
        isDraggingOver = false;
        if (event.payload.paths && event.payload.paths.length > 0) {
          handleDroppedFiles(event.payload.paths);
        }
      } else if (event.payload.type === "leave") {
        isDraggingOver = false;
      }
    }).then((fn) => {
      if (!activeListener) fn();
      else unlistenDD = fn;
    }).catch(console.error);

    return () => {
      activeListener = false;
      window.removeEventListener('keydown', handleKeydown);
      if (unlistenDD) unlistenDD();
      if (undoDebounceTimer) clearTimeout(undoDebounceTimer);
    };
  });

  async function processFilesToLoad(paths: string[]) {
    if (paths.length === 2) {
      const [fileA, fileB] = paths;
      targetPath = "";
      targetSubs = [];
      sourcePath = "";
      sourceSubs = [];
      await loadTarget(fileA);
      await loadSource(fileB);
    } else if (paths.length === 1) {
      const [fileA] = paths;
      targetPath = "";
      targetSubs = [];
      sourcePath = "";
      sourceSubs = [];
      await loadTarget(fileA);
      await tryAutoSelectSourceForTarget(fileA);
    }
  }

  function handleDroppedFiles(paths: string[]) {
    const srtPaths = paths.filter(p => p.toLowerCase().endsWith('.srt')).slice(-2);
    if (srtPaths.length === 0) return;

    const hasExistingFiles = !!(targetPath || sourcePath);
    if (hasExistingFiles) {
      pendingDroppedPaths = srtPaths;
      showOverwriteConfirm = true;
    } else {
      processFilesToLoad(srtPaths);
    }
  }

  function confirmOverwrite() {
    showOverwriteConfirm = false;
    if (pendingDroppedPaths.length > 0) {
      processFilesToLoad(pendingDroppedPaths);
      pendingDroppedPaths = [];
    }
  }

  function parseSrt(content: string): Subtitle[] {
    const blocks = content.trim().replace(/\r\n/g, '\n').split(/\n\s*\n/).filter(Boolean);
    return blocks.map(block => {
      const lines = block.split('\n');
      const id = parseInt(lines[0], 10) || 0;
      const timeLine = lines[1] || '';
      const times = timeLine.split(' --> ');
      const text = lines.slice(2).join('\n');
      const start = times[0] || '00:00:00,000';
      const end = times[1] || '00:00:00,000';
      return { id, start, end, text, originalStart: start, originalEnd: end };
    });
  }

  function serializeSrt(subs: Subtitle[]): string {
    return subs.map(s => `${s.id}\n${s.start} --> ${s.end}\n${s.text}`).join('\n\n') + '\n';
  }

  function normalizeAlignments() {
    // 1. Restore original timestamps for any existing subtitles in both lists
    targetSubs.forEach(s => {
      if (s.originalStart !== undefined) s.start = s.originalStart;
      if (s.originalEnd !== undefined) s.end = s.originalEnd;
    });
    sourceSubs.forEach(s => {
      if (s.originalStart !== undefined) s.start = s.originalStart;
      if (s.originalEnd !== undefined) s.end = s.originalEnd;
    });

    // 2. Collect unique maps of the original subtitles
    const targetMap = new Map(targetSubs.map(s => [s.id, s]));
    const sourceMap = new Map(sourceSubs.map(s => [s.id, s]));
    
    // Create a Set of all IDs, sort numerically
    const allIds = Array.from(new Set([...targetMap.keys(), ...sourceMap.keys()])).sort((a, b) => a - b);
    
    // Rebuild targetSubs (the original/primary side)
    targetSubs = allIds.map(id => {
      if (targetMap.has(id)) {
        return targetMap.get(id)!;
      }
      // If missing in target, inject a dummy object with source's original timestamps
      const s = sourceMap.get(id);
      const start = s?.originalStart || s?.start || '00:00:00,000';
      const end = s?.originalEnd || s?.end || '00:00:00,000';
      return { 
        id, 
        start, 
        end, 
        text: '', 
        originalStart: start, 
        originalEnd: end 
      };
    });

    // Rebuild sourceSubs (the secondary/editable side)
    const alignedTargetMap = new Map(targetSubs.map(s => [s.id, s]));

    sourceSubs = allIds.map(id => {
      const t = alignedTargetMap.get(id)!;
      const alignedStart = t.start;
      const alignedEnd = t.end;

      if (sourceMap.has(id)) {
        const s = sourceMap.get(id)!;
        s.start = alignedStart;
        s.end = alignedEnd;
        return s;
      }
      // If missing in source, inject an empty editable item
      return { 
        id, 
        start: alignedStart, 
        end: alignedEnd, 
        text: '',
        originalStart: alignedStart,
        originalEnd: alignedEnd
      };
    });
  }

  async function loadTarget(path: string) {
    try {
      // Encoding rilevato lato Rust (BOM, UTF-8/16, code page legacy):
      // readTextFile del plugin fs fallirebbe sui file non UTF-8.
      const content = await invoke<string>('read_subtitle_file', { path });
      sourcePath = "";
      sourceSubs = [];
      targetSubs = parseSrt(content);
      targetPath = path;
      normalizeAlignments();
      hasUnsavedChanges = false;
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
      const content = await invoke<string>('read_subtitle_file', { path });
      sourceSubs = parseSrt(content);
      sourcePath = path;
      normalizeAlignments();
      undoStack = []; // Reset undo on new file load
      hasUnsavedChanges = false;
      error = "";
      addActivityLog(`Source loaded: ${getFileName(path)} (${sourceSubs.length} subtitles)`, 'success');
    } catch (e) {
      error = `Error loading source: ${e}`;
      addActivityLog(`Source load failed: ${e}`, 'error');
    }
  }

  function checkUnsavedAndRun(action: () => void) {
    if (hasUnsavedChanges) {
      pendingBrowseAction = action;
      showUnsavedWarning = true;
    } else {
      action();
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
        hasUnsavedChanges = false;
        addActivityLog(`Aligned file saved: ${getFileName(savePath)}`, 'success');
        setTimeout(() => success = "", 3000);
      }
    } catch (e) {
      error = `Error saving file: ${e}`;
      addActivityLog(`Save failed: ${e}`, 'error');
    }
  }

  let isLoaded = $derived(targetSubs.length > 0 || sourceSubs.length > 0);
  let saveFileName = $derived(sourcePath ? getFileName(sourcePath).replace('.srt', '_aligned.srt') : '');

  let totalPages = $derived(isLoaded ? Math.ceil(Math.max(targetSubs.length, sourceSubs.length) / itemsPerPage) : 1);

  $effect(() => {
    if (currentPage >= totalPages) {
      currentPage = Math.max(0, totalPages - 1);
    }
  });
  
  // Array "a pettine" per iterare target/source fianco a fianco nella pagina
  // corrente. Prima, quando non c'era ancora nessun file caricato, veniva
  // sintetizzata una manciata di righe finte (id #1-#5, timestamp
  // 00:00:00,000): sembrava dato corrotto invece di un vero stato vuoto.
  // Ora semplicemente non ci sono righe, e la Content Grid mostra un
  // EmptyState come nelle altre tab.
  let currentPageItems = $derived(
    isLoaded
      ? Array.from({ length: itemsPerPage }, (_, i) => {
          const index = currentPage * itemsPerPage + i;
          return {
            index,
            target: targetSubs[index] || null,
            source: sourceSubs[index] || null
          };
        }).filter(item => item.target !== null || item.source !== null)
      : []
  );

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



</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div 
  role="region"
  aria-label="Revision content"
  class="h-full flex flex-col text-gray-200 bg-gray-900 relative overflow-hidden"
  onkeydown={handleKeydown}
  ondragover={(e) => {
    if (!active) return;
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'copy';
    }
    isDraggingOver = true;
  }}
  ondrop={(e) => {
    if (!active) return;
    e.preventDefault();
    isDraggingOver = false;
  }}
  ondragleave={(e) => {
    if (!active) return;
    const rt = e.relatedTarget as HTMLElement | null;
    const ct = e.currentTarget as HTMLElement;
    if (rt && ct.contains(rt)) return;
    isDraggingOver = false;
  }}
>
  {#if isDraggingOver}
    <div
      class="absolute inset-0 z-50 bg-teal-500/10 border-2 border-dashed border-teal-400 rounded-2xl flex items-center justify-center pointer-events-none"
    >
      <div class="text-center">
        <svg
          class="w-16 h-16 mx-auto mb-3 text-teal-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          ><path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
          /></svg
        >
        <p class="text-lg font-semibold text-teal-300">{t("align.dropOverlayTitle")}</p>
        <p class="text-xs text-teal-500 mt-1">{t("align.dropOverlaySubtitle")}</p>
      </div>
    </div>
  {/if}
  <div class="flex-1 overflow-y-auto p-6 min-h-0">
    <div class="min-h-full flex flex-col bg-white/[0.02] border border-white/5 rounded-2xl p-5 overflow-hidden gap-5">
      
      <!-- File Selection Area -->
      <div class="grid grid-cols-1 md:grid-cols-[1fr_auto_1fr] items-center gap-4 shrink-0 relative min-w-0 pb-4 border-b border-white/5">
      
        <!-- Target File -->
        <div class="flex flex-col gap-2 relative z-10 min-w-0">
          <div class="text-xs font-semibold text-gray-400 flex items-center gap-1.5">
            {#if targetFlag}<span class="text-sm">{targetFlag}</span>{/if}
            {t("align.baseSrt")}
            <span class="text-red-400 font-bold ml-0.5">*</span>
          </div>
          <PathPickerField
            value={targetPath}
            placeholder={t("align.dragDropSrt")}
            browseTitle={t("align.openSrt")}
            onexpand={() => expandedPathField = "target"}
            onbrowse={() => checkUnsavedAndRun(selectTarget)}
          />
          {#if targetSubs.length > 0}
            <div class="text-xs text-gray-400">{t("align.subtitlesLoaded", { count: targetSubs.length })}</div>
          {/if}
        </div>

        <!-- Swap Button -->
        <div class="flex items-center justify-center relative z-20 md:px-2 shrink-0 mt-4 md:mt-0">
          <button 
            onclick={swapFiles}
            disabled={!targetPath || !sourcePath}
            class="p-2.5 rounded-full border transition-colors group {(!targetPath || !sourcePath) ? 'bg-gray-900/40 text-gray-600 border-gray-800 cursor-not-allowed' : 'bg-gray-900/70 hover:bg-teal-500/20 text-gray-400 hover:text-teal-300 border-gray-700 hover:border-teal-500/50'}"
            title={t("align.swapFiles")}
          >
            <svg class="w-6 h-6 {!targetPath || !sourcePath ? '' : 'group-hover:rotate-180'} transition-transform duration-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
            </svg>
          </button>
        </div>

        <!-- Source File -->
        <div class="flex flex-col gap-2 relative z-10 min-w-0">
          <div class="text-xs font-semibold text-gray-400 flex items-center gap-1.5">
            {#if sourceFlag}<span class="text-sm">{sourceFlag}</span>{/if}
            {t("align.translationSrt")}
            <span class="text-red-400 font-bold ml-0.5">*</span>
          </div>
          <PathPickerField
            value={sourcePath}
            placeholder={t("align.dragDropSrt")}
            browseTitle={t("align.openSrt")}
            disabled={!targetPath}
            onexpand={() => targetPath && (expandedPathField = "source")}
            onbrowse={() => checkUnsavedAndRun(selectSource)}
          />
          {#if sourceSubs.length > 0}
            <div class="text-xs text-gray-400">{t("align.subtitlesLoaded", { count: sourceSubs.length })}</div>
          {/if}
        </div>
      </div>

      <!-- Editor Area -->
      <div class="flex-1 flex flex-col min-w-0 overflow-hidden">
        <!-- Pagination Top -->
        <div class="flex flex-col xl:flex-row xl:items-center xl:justify-between mb-4 gap-3 shrink-0">
          <div class="flex items-center gap-2 flex-wrap">
            <div class="min-w-[10.5rem] text-sm text-gray-400 font-medium bg-gray-800/80 px-3 py-1.5 rounded-md tabular-nums">
              {t("align.page")} <span class="inline-block min-w-[4ch] text-center text-white mx-1">{currentPage + 1}</span> {t("align.of")} <span class="inline-block min-w-[4ch] text-center text-white mx-1">{totalPages || 1}</span>
            </div>
            <div class="w-px h-6 bg-gray-700 mx-1"></div>
            <div class="flex items-center gap-1 bg-gray-800/50 px-1 py-1 rounded-md border border-gray-700">
              {#each ITEMS_PER_PAGE_OPTIONS as option, optionIndex}
                <button
                  onclick={() => setItemsPerPageIndex(optionIndex)}
                  disabled={!isLoaded}
                  class="min-w-8 rounded px-2 py-0.5 text-xs font-medium transition-colors {itemsPerPage === option
                    ? 'bg-teal-500/20 text-teal-200'
                    : 'text-gray-400 hover:bg-white/10 hover:text-gray-200'} disabled:opacity-40 disabled:cursor-not-allowed"
                  title={t("align.subsPerPage", { count: option })}
                >
                  {option}
                </button>
              {/each}
            </div>
          </div>
          <div class="flex gap-1.5 flex-wrap xl:justify-end">
            <button onclick={jumpStart} disabled={!isLoaded || currentPage === 0} class="btn-secondary px-3 py-1.5 disabled:opacity-50 flex items-center" title={t("align.goToStart")}>
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7" /></svg>
            </button>
            <!-- Jump to Prev Empty (orange glow) -->
            <button onclick={jumpToPrevEmpty} disabled={!isLoaded || !hasEmptyBackward} class="btn-secondary px-3 py-1.5 disabled:opacity-30 flex items-center empty-jump-btn" title={t("align.prevEmpty")}>
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7M22 19l-7-7 7-7" /></svg>
            </button>
            <button onclick={prevPage} disabled={!isLoaded || currentPage === 0} class="btn-secondary px-4 py-1.5 disabled:opacity-50 flex items-center">
              <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" /></svg>
              {t("align.prev")}
            </button>
            <button onclick={nextPage} disabled={!isLoaded || currentPage >= totalPages - 1} class="btn-secondary px-4 py-1.5 disabled:opacity-50 flex items-center">
              {t("align.next")}
              <svg class="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" /></svg>
            </button>
            <!-- Jump to Next Empty (orange glow) -->
            <button onclick={jumpToNextEmpty} disabled={!isLoaded || !hasEmptyForward} class="btn-secondary px-3 py-1.5 disabled:opacity-30 flex items-center empty-jump-btn" title={t("align.nextEmpty")}>
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M2 5l7 7-7 7" /></svg>
            </button>
            <button onclick={jumpEnd} disabled={!isLoaded || currentPage >= totalPages - 1} class="btn-secondary px-3 py-1.5 disabled:opacity-50 flex items-center" title={t("align.goToEnd")}>
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" /></svg>
            </button>
          </div>
        </div>

        <!-- Content Grid -->
        {#if !isLoaded}
          <EmptyState
            title={t("align.loadPrompt")}
            description={t("align.dragDropAnywhere")}
            iconPath="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"
          />
        {:else}
        <div class="pr-3 pl-1 space-y-6 custom-scrollbar pb-4 min-w-0 overflow-y-auto flex-1">
          {#each currentPageItems as item (item.index)}
            {@const isMissingPair = targetPath && sourcePath && (
              (!!item.source && (!item.source.text || item.source.text.trim() === '')) ||
              (!!item.target && (!item.target.text || item.target.text.trim() === ''))
            )}
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
                    class="flex-1 w-full bg-transparent p-3 text-sm text-gray-300 resize-none min-h-[90px] focus:outline-none min-w-0 placeholder-gray-600/70"
                    readonly
                    value={item.target.text}
                    placeholder={!isLoaded ? (t("align.waitingOriginal") || "In attesa del file originale...") : ""}
                    onwheel={handleTextareaWheel}
                  ></textarea>
                {:else}
                  <div class="h-full min-h-[125px] flex flex-col items-center justify-center text-gray-600 text-sm">
                    <svg class="w-6 h-6 mb-2 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" /></svg>
                    <span>{t("align.noSubtitle")}</span>
                  </div>
                {/if}
              </div>

              <!-- Source Side (Editable) -->
              <div class="flex flex-col h-full rounded-lg border bg-indigo-950/10 focus-within:border-indigo-500/50 transition-colors overflow-hidden relative min-w-0 {isMissingPair ? 'border-orange-500/60' : 'border-indigo-500/20'}">
                {#if isLoaded && item.source}
                  <!-- Header part -->
                  <div class="flex justify-between items-center text-xs text-indigo-400/70 bg-indigo-950/40 px-3 py-2 border-b border-indigo-500/20 font-mono tracking-wider truncate">
                    <span class="bg-indigo-900/50 flex-shrink-0 text-indigo-300 px-2 py-0.5 rounded max-w-full">#{item.source.id}</span>
                    <span class="flex items-center gap-1.5 md:gap-2 truncate ml-2">
                      <span class="text-blue-400/70 truncate min-w-0">{item.source.start}</span>
                      <svg class="w-3 h-3 mx-0.5 text-indigo-500/50 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/></svg>
                      <span class="text-indigo-400/70 truncate min-w-0">{item.source.end}</span>
                    </span>
                  </div>
                  <!-- Content part -->
                  <textarea 
                    class="flex-1 w-full bg-transparent p-3 text-[15px] leading-relaxed text-indigo-100 resize-none min-h-[90px] focus:outline-none placeholder-indigo-900/50 min-w-0"
                    bind:value={sourceSubs[item.index].text}
                    oninput={scheduleUndo}
                    onwheel={handleTextareaWheel}
                    placeholder={t("align.typeSubtitle")}
                  ></textarea>
                {:else}
                  <div class="flex flex-col h-full min-w-0">
                    <div class="flex justify-between items-center text-xs text-indigo-400/30 bg-indigo-950/20 px-3 py-2 border-b border-indigo-500/10 font-mono tracking-wider truncate">
                      <span class="bg-indigo-900/20 flex-shrink-0 text-indigo-300/30 px-2 py-0.5 rounded max-w-full">#{item.index + 1}</span>
                      <span class="flex items-center gap-1.5 md:gap-2 truncate ml-2">
                        <span class="text-blue-400/20 truncate min-w-0">00:00:00,000</span>
                        <svg class="w-3 h-3 mx-0.5 text-indigo-500/20 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/></svg>
                        <span class="text-indigo-400/20 truncate min-w-0">00:00:00,000</span>
                      </span>
                    </div>
                    <textarea 
                      class="flex-1 w-full bg-transparent p-3 text-[15px] leading-relaxed text-indigo-100/30 resize-none min-h-[90px] focus:outline-none placeholder-indigo-900/30 min-w-0 cursor-not-allowed"
                      disabled
                      value=""
                      placeholder={t("align.waitingTranslation") || "In attesa del file di traduzione..."}
                    ></textarea>
                  </div>
                {/if}
              </div>
              
            </div>
          {/each}
        </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Fixed Bottom Band with Action Buttons -->
  <FooterActions justify="center">
    {#snippet left()}
      <div class="flex-1 w-full grid grid-cols-2">
        <!-- Left Column: Save Button aligned to the right of the left half -->
        <div class="flex items-center justify-end pr-3">
          <div class="relative group">
            <button
              onclick={saveSource}
              disabled={sourceSubs.length === 0}
              class="px-5 py-2.5 bg-emerald-600/80 hover:bg-emerald-500/80 border border-emerald-500/30 disabled:bg-emerald-600/40 text-emerald-100 rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-55 cursor-pointer"
            >
              <svg class="w-4 h-4 text-emerald-100" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
              </svg>
              {t("align.saveResult")}
            </button>
            <div class="pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 z-50 mb-3 rounded-xl border border-emerald-500/30 bg-gray-950/95 p-3 text-center text-xs text-emerald-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
              {sourceSubs.length === 0 ? "Carica e allinea i file per salvare" : t("align.saveResult")}
            </div>
          </div>
        </div>

        <!-- Right Column: Filename Label aligned to the left of the right half -->
        <div class="flex items-center justify-start pl-3">
          <div class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-semibold select-none
            {sourceSubs.length > 0 
              ? 'border-indigo-500/40 bg-indigo-500/10 text-indigo-300' 
              : 'border-gray-700/40 bg-gray-800/10 text-gray-500/60'}"
          >
            <svg class="w-3.5 h-3.5 {sourceSubs.length > 0 ? 'text-indigo-300' : 'text-gray-500/50'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            <span class="truncate max-w-[280px] md:max-w-[400px]" title={sourcePath ? saveFileName : ""}>
              {sourcePath ? saveFileName : "..."}
            </span>
          </div>
        </div>
      </div>
    {/snippet}
  </FooterActions>

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "target" ? t("align.baseSrt") : t("align.translationSrt")}
    value={expandedPathField === "target" ? targetPath || "—" : sourcePath || "—"}
    onclose={() => (expandedPathField = null)}
  />

  <ConfirmDialog
    show={showOverwriteConfirm}
    title="Sovrascrivere i file esistenti?"
    message="Hai già dei file caricati in questa sessione. Se procedi, i dati correnti verranno sostituiti con quelli nuovi."
    confirmText="Sovrascrivi"
    cancelText="Annulla"
    variant="warning"
    on:cancel={() => {
      showOverwriteConfirm = false;
      pendingDroppedPaths = [];
    }}
    on:confirm={confirmOverwrite}
  />

  <ConfirmDialog
    show={showUnsavedWarning}
    title="Modifiche non salvate"
    message="Hai delle modifiche non salvate ai sottotitoli. Se procedi e carichi dei nuovi file, le modifiche correnti verranno perse permanentemente."
    confirmText="Procedi comunque"
    cancelText="Annulla"
    variant="warning"
    on:cancel={() => {
      showUnsavedWarning = false;
      pendingBrowseAction = null;
    }}
    on:confirm={() => {
      showUnsavedWarning = false;
      if (pendingBrowseAction) {
        pendingBrowseAction();
        pendingBrowseAction = null;
      }
    }}
  />
</div>

<style>
  .custom-scrollbar {
    overscroll-behavior-y: auto;
  }

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
