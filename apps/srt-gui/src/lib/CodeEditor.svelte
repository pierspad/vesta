<script lang="ts">
  import { onDestroy } from "svelte";
  import { locale } from "./i18n";
  import { snackbar } from "./snackbarStore.svelte";

  let t = $derived($locale);

  let {
    value = $bindable(""),
    language = "html",
    onchange = () => {},
    heightClass = "h-64",
    readonly = false,
    placeholder = "",
    id = "",
    class: classProp = "",
    textareaClass = "",
    wrap = false,
    lineNumbers = !wrap,
  } = $props();

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
    if (contextMenu) {
      const key = e.key.toLowerCase();
      if (e.key === "Escape") {
        closeContextMenu();
        e.preventDefault();
        e.stopPropagation();
        return;
      }
      if (key === "u" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        e.stopPropagation();
        undo();
        closeContextMenu();
        return;
      }
      if (key === "r" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        e.stopPropagation();
        redo();
        closeContextMenu();
        return;
      }
      if (key === "c" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        e.stopPropagation();
        void copySelection();
        return;
      }
      if (key === "x" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        e.stopPropagation();
        void cutSelection();
        return;
      }
      if (key === "v" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        e.stopPropagation();
        void pasteClipboard();
        return;
      }
      if (key === "a" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        e.stopPropagation();
        selectAllText();
        return;
      }
    }

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

  let copied = $state(false);
  let copyTimeout: ReturnType<typeof setTimeout> | null = null;

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(value || "");
      copied = true;
      if (copyTimeout) clearTimeout(copyTimeout);
      copyTimeout = setTimeout(() => {
        copied = false;
      }, 2000);
      snackbar.show(t("settings.keyCopied") || "Copied to clipboard", "success");
    } catch (err) {
      console.error("Failed to copy text: ", err);
      snackbar.show("Failed to copy", "error");
    }
  }

  onDestroy(() => {
    closeContextMenu();
    if (copyTimeout) clearTimeout(copyTimeout);
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

  function escapeHtml(text: string) {
    return text
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  function highlightJsonLike(code: string) {
    let html = "";
    let i = 0;

    while (i < code.length) {
      const char = code[i];
      const next = code[i + 1];

      if (char === "/" && next === "/") {
        const end = code.indexOf("\n", i);
        const comment = end === -1 ? code.slice(i) : code.slice(i, end);
        html += `<span class="text-slate-500">${escapeHtml(comment)}</span>`;
        i += comment.length;
        continue;
      }

      if (char === "/" && next === "*") {
        const end = code.indexOf("*/", i + 2);
        const comment = end === -1 ? code.slice(i) : code.slice(i, end + 2);
        html += `<span class="text-slate-500">${escapeHtml(comment)}</span>`;
        i += comment.length;
        continue;
      }

      if (char === '"') {
        let end = i + 1;
        let escaped = false;
        while (end < code.length) {
          const current = code[end];
          if (current === '"' && !escaped) {
            end += 1;
            break;
          }
          escaped = current === "\\" && !escaped;
          if (current !== "\\") escaped = false;
          end += 1;
        }
        const token = code.slice(i, end);
        let lookahead = end;
        while (/\s/.test(code[lookahead] || "")) lookahead += 1;
        const className = code[lookahead] === ":" ? "text-sky-300" : "text-emerald-300";
        html += `<span class="${className}">${escapeHtml(token)}</span>`;
        i = end;
        continue;
      }

      const keyword = code.slice(i).match(/^(true|false|null)\b/);
      if (keyword) {
        html += `<span class="text-violet-300">${keyword[0]}</span>`;
        i += keyword[0].length;
        continue;
      }

      const number = code.slice(i).match(/^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/);
      if (number) {
        html += `<span class="text-amber-300">${number[0]}</span>`;
        i += number[0].length;
        continue;
      }

      if (/[\[\]{}:,]/.test(char)) {
        html += `<span class="text-pink-400">${escapeHtml(char)}</span>`;
        i += 1;
        continue;
      }

      html += escapeHtml(char);
      i += 1;
    }

    return html;
  }

  // Basic highlight
  function highlight(code: string, lang: string) {
    if (!code) return "";
    let html = escapeHtml(code);
    
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
    } else if (lang === "json" || lang === "jsonc") {
      html = highlightJsonLike(code);
    } else if (lang === "markdown" || lang === "prompt") {
      // Template variables: {{front}}, {{back}}, {{notes}} or others like {{expression}}, {{meaning}}
      html = html.replace(/(\{\{[a-zA-Z0-9_]+\}\})/g, '<span class="text-violet-400 font-bold bg-violet-500/10 px-1 border border-violet-500/20 rounded">$1</span>');
      // Inline HTML tags in markdown
      html = html.replace(/(&lt;\/?[a-zA-Z0-9:-]+.*?&gt;)/g, '<span class="text-purple-400">$1</span>');
      // Headings
      html = html.replace(/^(#+\s+.*)$/gm, '<span class="text-blue-400 font-bold">$1</span>');
      // Bold
      html = html.replace(/(\*\*.*?\*\*)/g, '<span class="text-gray-100 font-bold">$1</span>');
      // Inline code
      html = html.replace(/(`.*?`)/g, '<span class="text-emerald-400 bg-emerald-500/10 px-0.5 rounded">$1</span>');
      // List bullets
      html = html.replace(/^(\s*[-*+•]\s+)/gm, '<span class="text-amber-400 font-bold">$1</span>');
      html = html.replace(/^(\s*\d+\.\s+)/gm, '<span class="text-amber-400 font-bold">$1</span>');
    }
    // ensure blank line at the end renders spaces correctly if needed
    return html;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class={`relative flex w-full ${heightClass} bg-gray-900 rounded-lg overflow-hidden border border-white/10 group focus-within:border-indigo-500/50 transition-colors ${classProp}`}
>
  <!-- Line Numbers -->
  {#if lineNumbers}
  <div 
    onclick={() => textareaElement?.focus()}
    class="w-10 bg-black/40 py-3 text-right pr-2 text-sm font-mono text-gray-600 select-none overflow-hidden shrink-0 cursor-text"
  >
    <div style="transform: translateY(-{scrollTop}px)">
      {#each lines as _, i}
        <div class="leading-relaxed whitespace-pre">{i + 1}</div>
      {/each}
    </div>
  </div>
  {/if}

  <div class="relative flex-1 overflow-hidden h-full">
    <!-- Highlighted Code -->
    <pre class="absolute w-full pt-3 pb-3 pl-3 pr-9 m-0 font-mono text-sm leading-relaxed text-gray-300 pointer-events-none {wrap ? 'whitespace-pre-wrap break-words' : 'whitespace-pre break-normal'}" aria-hidden="true" style="transform: translate(-{wrap ? 0 : scrollLeft}px, -{scrollTop}px);">{@html highlight(value, language)}<br/></pre>
    
    <!-- Transparent Textarea -->
    <textarea
      bind:this={textareaElement}
      id={id}
      bind:value
      readonly={readonly}
      placeholder={placeholder}
      oninput={handleInput}
      onkeydown={handleKeydown}
      oncontextmenu={openContextMenu}
      onscroll={(e) => {
        scrollTop = e.currentTarget.scrollTop;
        if (!wrap) scrollLeft = e.currentTarget.scrollLeft;
      }}
      wrap={wrap ? "soft" : "off"}
      class="absolute inset-0 w-full h-full p-3 m-0 font-mono text-sm leading-relaxed text-transparent bg-transparent border-none resize-none outline-none caret-white pr-9 custom-scrollbar {wrap ? 'whitespace-pre-wrap break-words overflow-x-hidden' : 'whitespace-pre break-normal'} {textareaClass}"
      spellcheck="false"
    ></textarea>

    <!-- Copy Button -->
    <button
      type="button"
      onclick={handleCopy}
      class="absolute top-2.5 right-2.5 z-10 w-8 h-8 rounded-lg bg-gray-900/80 hover:bg-cyan-500/20 text-gray-400 hover:text-cyan-200 border border-white/10 hover:border-cyan-500/30 transition-all flex items-center justify-center cursor-pointer shadow-md opacity-60 hover:opacity-100"
      title={t("common.copy") || "Copy"}
    >
      {#if copied}
        <svg class="w-4 h-4 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        </svg>
      {:else}
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
        </svg>
      {/if}
    </button>
  </div>

  {#if contextMenu}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="fixed inset-0 z-[80]" onmousedown={closeContextMenu} oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}>
      <div
        class="vesta-context-menu"
        style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
        onmousedown={(e) => e.stopPropagation()}
      >
        <button type="button" class="vesta-context-menu-item" onclick={() => { undo(); closeContextMenu(); }}>
          <span>{t("common.undo")}</span>
          <kbd>{t("shortcuts.hint.undo")}</kbd>
        </button>
        <button type="button" class="vesta-context-menu-item" onclick={() => { redo(); closeContextMenu(); }}>
          <span>{t("common.redo")}</span>
          <kbd>{t("shortcuts.hint.redo")}</kbd>
        </button>
        <div class="vesta-context-menu-separator"></div>
        <button type="button" class="vesta-context-menu-item" onclick={copySelection}>
          <span>{t("common.copy")}</span>
          <kbd>{t("shortcuts.hint.copy")}</kbd>
        </button>
        <button type="button" class="vesta-context-menu-item" onclick={cutSelection}>
          <span>{t("common.cut")}</span>
          <kbd>{t("shortcuts.hint.cut")}</kbd>
        </button>
        <button type="button" class="vesta-context-menu-item" onclick={pasteClipboard}>
          <span>{t("common.paste")}</span>
          <kbd>{t("shortcuts.hint.paste")}</kbd>
        </button>
        <div class="vesta-context-menu-separator"></div>
        <button type="button" class="vesta-context-menu-item" onclick={selectAllText}>
          <span>{t("common.selectAll")}</span>
          <kbd>{t("shortcuts.hint.selectAll")}</kbd>
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
