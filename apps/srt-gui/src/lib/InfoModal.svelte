<script lang="ts">
  /**
   * InfoModal — Reusable help/info modal used across all tabs.
   *
   * Usage:
   *   <InfoModal section={helpSection} onclose={() => helpSection = null} {sections} />
   *
   * The `sections` prop maps each panel ID to { titleKey, contentKey }.
   * Both keys are looked up via t() from the i18n system.
   */

  import { t } from "./i18n";

  interface InfoSection {
    /** i18n key for the modal title */
    titleKey: string;
    /** i18n key for the HTML body content */
    contentKey: string;
  }

  interface Props {
    /** Currently active section ID, or null if closed */
    section: string | null;
    /** Close callback */
    onclose: () => void;
    /** Map of section IDs → title/content i18n keys */
    sections: Record<string, InfoSection>;
    /** Optional max-width class override (default: max-w-2xl) */
    maxWidth?: string;
  }

  let { section, onclose, sections, maxWidth = "max-w-2xl" }: Props = $props();

  let currentSection = $derived(section ? sections[section] : null);
</script>

{#if section && currentSection}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={onclose}
    onkeydown={(e) => {
      if (e.key === "Escape") onclose();
    }}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="bg-gray-900 border border-gray-700 rounded-xl w-full {maxWidth} flex flex-col"
      style="max-height: min(90vh, 700px);"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-6 pt-5 pb-3 flex-shrink-0">
        <h2 class="text-lg font-bold text-white">
          {t(currentSection.titleKey)}
        </h2>
        <button
          onclick={onclose}
          class="text-gray-400 hover:text-white text-xl leading-none"
          aria-label="Close"
        >✕</button>
      </div>
      <!-- Scrollable content -->
      <div
        class="text-gray-300 text-sm leading-relaxed overflow-y-auto help-content px-6 flex-1 min-h-0"
      >
        {@html t(currentSection.contentKey)}
      </div>
      <!-- Footer -->
      <div class="px-6 py-4 flex justify-end flex-shrink-0 border-t border-white/5">
        <button
          onclick={onclose}
          class="btn-primary py-1.5 px-4 text-sm"
        >{t("common.close")}</button>
      </div>
    </div>
  </div>
{/if}
