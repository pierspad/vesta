<script lang="ts">
  import { renderMarkdown } from "$lib/utils/renderMarkdown";
  let { text }: { text: string } = $props();
  let html = $derived(renderMarkdown(text));
</script>

<div class="markdown-content">{@html html}</div>

<style>
  .markdown-content { color: #cbd5e1; font-size: 0.875rem; line-height: 1.75; overflow-wrap: anywhere; }
  .markdown-content :global(:is(h1, h2, h3, h4, h5, h6)) { color: #e5e7eb; font-weight: 650; line-height: 1.4; margin: 1.5em 0 0.65em; }
  .markdown-content :global(h1) { font-size: 1.5rem; }
  .markdown-content :global(h2) { font-size: 1.25rem; }
  .markdown-content :global(h3) { font-size: 1rem; }
  .markdown-content :global(:first-child) { margin-top: 0; }
  .markdown-content :global(p) { margin: 0.75em 0; }
  .markdown-content :global(ul) { list-style: disc; padding-inline-start: 1.5rem; margin: 0.75em 0; }
  .markdown-content :global(ol) { list-style: decimal; padding-inline-start: 1.5rem; margin: 0.75em 0; }
  .markdown-content :global(li) { padding-inline-start: 0.25rem; margin: 0.4em 0; }
  .markdown-content :global(strong) { color: #e5e7eb; font-weight: 650; }
  .markdown-content :global(a) { color: #a5b4fc; text-decoration: underline; text-underline-offset: 3px; }
  .markdown-content :global(a:hover) { color: #c7d2fe; }
  .markdown-content :global(a:focus-visible) { outline: 2px solid #818cf8; outline-offset: 3px; border-radius: 2px; }
  .markdown-content :global(code) { border-radius: 4px; background: rgb(255 255 255 / 0.06); padding: 0.15em 0.35em; font-size: 0.85em; color: #c4b5fd; }
  .markdown-content :global(pre) { overflow-x: auto; border: 1px solid rgb(255 255 255 / 0.08); border-radius: 10px; background: #0b1220; padding: 1rem; margin: 1em 0; }
  .markdown-content :global(pre code) { background: none; padding: 0; color: inherit; }
  .markdown-content :global(blockquote) { border-inline-start: 3px solid #6366f1; padding-inline-start: 1rem; color: #94a3b8; margin: 1em 0; }
  .markdown-content :global(hr) { border: 0; border-top: 1px solid rgb(255 255 255 / 0.1); margin: 1.5em 0; }
  .markdown-content :global(table) { display: block; max-width: 100%; overflow-x: auto; border-collapse: collapse; margin: 1em 0; }
  .markdown-content :global(:is(th, td)) { border: 1px solid rgb(255 255 255 / 0.1); padding: 0.5rem 0.75rem; text-align: start; }
  .markdown-content :global(th) { background: rgb(255 255 255 / 0.04); color: #e5e7eb; font-weight: 600; }
</style>
