<script lang="ts">
  import { locale } from "$lib/i18n";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import type { InstalledPlugin } from "$lib/services/pluginClient";
  import { resolveHintAndLink } from "./pluginHelpers";

  interface Props {
    plugin: InstalledPlugin;
    values: Record<string, any>;
    onClose: () => void;
    onSave: () => void;
  }

  let { plugin, values = $bindable(), onClose, onSave }: Props = $props();
  let t = $derived($locale);
  let revealedPasswords = $state<Record<string, boolean>>({});
</script>

{#if plugin.manifest.settings}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
    <div class="w-full max-w-lg rounded-2xl border border-white/10 bg-gray-900 p-6 flex flex-col gap-5 shadow-2xl">
      <div class="flex items-center justify-between border-b border-white/10 pb-3">
        <h3 class="text-base font-bold text-white">{plugin.name} — {t("common.settings")}</h3>
        <button
          type="button"
          class="text-gray-400 hover:text-white cursor-pointer p-1"
          aria-label={t("common.close")}
          onclick={onClose}
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
        </button>
      </div>

      <div class="flex flex-col gap-4 max-h-[60vh] overflow-y-auto scrollbar-thin">
        {#each plugin.manifest.settings as field}
          {#if !field.expertOnly || uiMode.expertMode}
            {#if field.type === "checkbox"}
              <button
                type="button"
                role="checkbox"
                aria-checked={Boolean(values[field.id])}
                class="flex items-center gap-3 p-3 rounded-xl border transition-all text-left cursor-pointer {Boolean(values[field.id]) ? 'border-indigo-500/50 bg-indigo-500/10 text-white' : 'border-white/10 bg-white/[0.03] text-gray-300 hover:bg-white/[0.06]'}"
                onclick={() => values[field.id] = !values[field.id]}
              >
                <div class="w-5 h-5 rounded-lg border flex items-center justify-center transition-colors {Boolean(values[field.id]) ? 'border-indigo-400 bg-indigo-600 text-white' : 'border-white/20 bg-black/20 text-transparent'}">
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"/></svg>
                </div>
                <div class="flex flex-col">
                  <span class="text-sm font-medium">{field.label}</span>
                  {#if field.description}
                    <span class="text-xs text-gray-400">{field.description}</span>
                  {/if}
                </div>
              </button>
            {:else}
              {@const info = resolveHintAndLink(field.hint, field.helpText ?? field.description, field.link)}
              <div class="flex flex-col gap-1.5">
                <label for="modal-setting-{field.id}" class="text-xs font-medium text-gray-400">{field.label}</label>
                <div class="relative w-full">
                  <input
                    id="modal-setting-{field.id}"
                    type={field.type === "password" ? (revealedPasswords[field.id] ? "text" : "password") : field.type === "number" ? "number" : "text"}
                    bind:value={values[field.id]}
                    placeholder={field.description ?? ""}
                    class="h-10 px-3 {field.type === 'password' ? 'pr-10' : ''} w-full rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors"
                  />
                  {#if field.type === "password"}
                    <button
                      type="button"
                      aria-pressed={!!revealedPasswords[field.id]}
                      aria-label={t("settings.toggleVisibility")}
                      title={t("settings.toggleVisibility")}
                      onclick={() => { revealedPasswords[field.id] = !revealedPasswords[field.id]; }}
                      class="absolute inset-y-0 right-0 flex items-center pr-3 text-gray-400 hover:text-gray-200 transition-colors cursor-pointer"
                    >
                      {#if revealedPasswords[field.id]}
                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
                        </svg>
                      {:else}
                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                        </svg>
                      {/if}
                    </button>
                  {/if}
                </div>
                {#if info.text || info.link}
                  <div class="flex flex-wrap items-center gap-2 mt-0.5">
                    {#if info.text}
                      <span class="text-xs text-gray-500">{info.text}</span>
                    {/if}
                    {#if info.link}
                      <a
                        href={info.link.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        onclick={(e) => { e.preventDefault(); void openUrl(info.link!.url); }}
                        class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-semibold transition-colors cursor-pointer"
                      >
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                        {info.link.label}
                      </a>
                    {/if}
                  </div>
                {/if}
              </div>
            {/if}
          {/if}
        {/each}
      </div>

      <div class="flex justify-end gap-3 pt-3 border-t border-white/10">
        <button
          type="button"
          class="btn-secondary text-xs px-4 py-2 cursor-pointer"
          onclick={onClose}
        >
          {t("common.cancel")}
        </button>
        <button
          type="button"
          class="btn-primary text-xs px-5 py-2 font-semibold cursor-pointer"
          onclick={onSave}
        >
          {t("plugins.save")}
        </button>
      </div>
    </div>
  </div>
{/if}
