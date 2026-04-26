<script lang="ts">
  import { onDestroy } from "svelte";

  let { value = $bindable(""), language = "html", onchange = () => {} } = $props();

  let history = $state<string[]>([value || ""]);
  let historyIndex = $state(0);
  let isUndoRedo = false;
  let textareaElement = $state<HTMLTextAreaElement | null>(null);
  let contextMenu = $state<{ x: number; y: number } | null>(null);

  function pushHistory(newValue: string) {
    if (isUndoRedo) {
      isUndoRedo = false;
      return;
    }
    if (history[historyIndex] === newValue) return;
    
    // truncate future history if we are in the middle and making a new edit
    history = history.slice(0, historyIndex + 1);
    history.push(newValue);
    if (history.length > 100) history.shift(); // limit history size
    historyIndex = history.length - 1;
  }

  function undo() {
    if (historyIndex > 0) {
      isUndoRedo = true;
      historyIndex--;
      value = history[historyIndex];
      onchange();
    }
  }

  function redo() {
    if (historyIndex < history.length - 1) {
      isUndoRedo = true;
      historyIndex++;
      value = history[historyIndex];
      onchange();
    }
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function openContextMenu(e: MouseEvent) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY };
  }

  function replaceSelection(replacement: string) {
    if (!textareaElement) return;
    const start = textareaElement.selectionStart;
    const end = textareaElement.selectionEnd;
    value = `${value.slice(0, start)}${replacement}${value.slice(end)}`;
    pushHistory(value);
    onchange();
    setTimeout(() => {
      if (!textareaElement) return;
      const cursor = start + replacement.length;
      textareaElement.selectionStart = textareaElement.selectionEnd = cursor;
      textareaElement.focus();
    }, 0);
  }

  function selectedText() {
    if (!textareaElement) return "";
    return value.slice(textareaElement.selectionStart, textareaElement.selectionEnd);
  }

  async function copySelection() {
    const text = selectedText() || value;
    await navigator.clipboard.writeText(text);
    closeContextMenu();
  }

  async function cutSelection() {
    const text = selectedText();
    if (!text) return copySelection();
    await navigator.clipboard.writeText(text);
    replaceSelection("");
    closeContextMenu();
  }

  async function pasteClipboard() {
    try {
      replaceSelection(await navigator.clipboard.readText());
    } finally {
      closeContextMenu();
    }
  }

  function selectAllText() {
    textareaElement?.focus();
    textareaElement?.select();
    closeContextMenu();
  }

  function handleKeydown(e: KeyboardEvent) {
    // Check for Ctrl+Z and Ctrl+Y / Ctrl+Shift+Z
    if (e.ctrlKey || e.metaKey) {
      if (e.key === 'z') {
        e.preventDefault();
        if (e.shiftKey) {
          redo();
        } else {
          undo();
        }
      } else if (e.key === 'y') {
        e.preventDefault();
        redo();
      }
    }
    
    // Auto-indent support on Tab (optional, but good for code editors)
    if (e.key === 'Tab') {
      e.preventDefault();
      const target = e.target as HTMLTextAreaElement;
      const start = target.selectionStart;
      const end = target.selectionEnd;
      
      const v = value || "";
      const before = v.substring(0, start);
      const after = v.substring(end);
      
      value = before + "  " + after;
      
      // Update cursor position after Svelte updates DOM
      setTimeout(() => {
        target.selectionStart = target.selectionEnd = start + 2;
        pushHistory(value);
      }, 0);
    }
  }

  function handleInput(e: Event) {
    value = (e.target as HTMLTextAreaElement).value;
    pushHistory(value);
    onchange();
  }

  onDestroy(() => {
    closeContextMenu();
  });

  let lines = $derived((value || "").split("\n"));
  let scrollTop = $state(0);
  let scrollLeft = $state(0);
  const variableClasses: Record<string, string> = {
    Expression: "var-token-expression",
    Meaning: "var-token-meaning",
    Reading: "var-token-reading",
    Audio: "var-token-audio",
    Snapshot: "var-token-snapshot",
    Video: "var-token-video",
    Tags: "var-token-tags",
    SequenceMarker: "var-token-sequence",
    Notes: "var-token-notes",
  };

  // Basic highlight
  function highlight(code: string, lang: string) {
    if (!code) return "";
    let html = code
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
    
    if (lang === "html") {
      html = html.replace(/([a-zA-Z-]+)="([^"]*)"/g, '<span class="text-amber-300">$1</span>="<span class="text-emerald-300">$2</span>"');
      html = html.replace(/(&lt;[\/\w:-]+)/g, '<span class="text-pink-400">$1</span>');
      html = html.replace(/(\/?&gt;)/g, '<span class="text-pink-400">$1</span>');
      html = html.replace(/\{\{(.*?)\}\}/g, (_match: string, variableName: string) => {
        const normalized = String(variableName).trim();
        const className = variableClasses[normalized] || "var-token-default";
        return `<span class="${className}">{{${normalized}}}</span>`;
      });
    } else if (lang === "css") {
      html = html.replace(/([\.#]?[\w-]+)\s*{/g, '<span class="text-pink-400">$1</span> {');
      html = html.replace(/([\w-]+)\s*:/g, '<span class="text-blue-300">$1</span>:');
      html = html.replace(/:\s*(.*?);/g, ': <span class="text-emerald-300">$1</span>;');
      html = html.replace(/\{\{(.*?)\}\}/g, (_match: string, variableName: string) => {
        const normalized = String(variableName).trim();
        const className = variableClasses[normalized] || "var-token-default";
        return `<span class="${className}">{{${normalized}}}</span>`;
      });
    }
    // ensure blank line at the end renders spaces correctly if needed
    return html;
  }
</script>

<div class="relative flex w-full h-64 bg-gray-900 rounded-lg overflow-hidden border border-white/10 group focus-within:border-indigo-500/50 transition-colors">
  <!-- Line Numbers -->
  <div class="w-10 bg-black/40 py-3 text-right pr-2 text-sm font-mono text-gray-600 select-none overflow-hidden shrink-0">
    <div style="transform: translateY(-{scrollTop}px)">
      {#each lines as _, i}
        <div class="leading-relaxed whitespace-pre">{i + 1}</div>
      {/each}
    </div>
  </div>

  <div class="relative flex-1 overflow-hidden h-full">
    <!-- Highlighted Code -->
    <pre class="absolute w-full p-3 m-0 font-mono text-sm leading-relaxed text-gray-300 whitespace-pre break-normal pointer-events-none" aria-hidden="true" style="transform: translate(-{scrollLeft}px, -{scrollTop}px);">{@html highlight(value, language)}<br/></pre>
    
    <!-- Transparent Textarea -->
    <textarea
      bind:this={textareaElement}
      bind:value
      oninput={handleInput}
      onkeydown={handleKeydown}
      oncontextmenu={openContextMenu}
      onscroll={(e) => {
        scrollTop = e.currentTarget.scrollTop;
        scrollLeft = e.currentTarget.scrollLeft;
      }}
      wrap="off"
      class="absolute inset-0 w-full h-full p-3 m-0 font-mono text-sm leading-relaxed text-transparent bg-transparent border-none resize-none outline-none caret-white whitespace-pre break-normal pr-9 custom-scrollbar"
      spellcheck="false"
    ></textarea>
  </div>

  {#if contextMenu}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="fixed inset-0 z-[80]" onmousedown={closeContextMenu} oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}>
      <div
        class="absolute min-w-[180px] overflow-hidden rounded-xl border border-white/10 bg-gray-900/98 py-1 shadow-2xl"
        style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
        onmousedown={(e) => e.stopPropagation()}
      >
        <button type="button" class="editor-menu-item" onclick={() => { undo(); closeContextMenu(); }}>
          <span>Undo</span>
          <kbd>Ctrl Z</kbd>
        </button>
        <button type="button" class="editor-menu-item" onclick={() => { redo(); closeContextMenu(); }}>
          <span>Redo</span>
          <kbd>Ctrl Y</kbd>
        </button>
        <div class="my-1 h-px bg-white/10"></div>
        <button type="button" class="editor-menu-item" onclick={copySelection}>
          <span>Copy</span>
          <kbd>Ctrl C</kbd>
        </button>
        <button type="button" class="editor-menu-item" onclick={cutSelection}>
          <span>Cut</span>
          <kbd>Ctrl X</kbd>
        </button>
        <button type="button" class="editor-menu-item" onclick={pasteClipboard}>
          <span>Paste</span>
          <kbd>Ctrl V</kbd>
        </button>
        <div class="my-1 h-px bg-white/10"></div>
        <button type="button" class="editor-menu-item" onclick={selectAllText}>
          <span>Select all</span>
          <kbd>Ctrl A</kbd>
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  textarea::selection {
    background-color: rgba(99, 102, 241, 0.4);
    color: transparent;
  }

  .editor-menu-item {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.55rem 0.8rem;
    color: rgb(209 213 219);
    font-size: 0.8125rem;
    transition: background-color 0.12s ease, color 0.12s ease;
  }

  .editor-menu-item:hover {
    background: rgba(255, 255, 255, 0.08);
    color: white;
  }

  .editor-menu-item kbd {
    color: rgb(107 114 128);
    font-size: 0.65rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
  }

  :global(.var-token-expression) { color: rgb(125 211 252); }
  :global(.var-token-meaning) { color: rgb(110 231 183); }
  :global(.var-token-reading) { color: rgb(196 181 253); }
  :global(.var-token-audio) { color: rgb(251 113 133); }
  :global(.var-token-snapshot) { color: rgb(251 191 36); }
  :global(.var-token-video) { color: rgb(251 146 60); }
  :global(.var-token-tags) { color: rgb(190 242 100); }
  :global(.var-token-sequence) { color: rgb(103 232 249); }
  :global(.var-token-notes) { color: rgb(240 171 252); }
  :global(.var-token-default) { color: rgb(129 140 248); }
</style>
