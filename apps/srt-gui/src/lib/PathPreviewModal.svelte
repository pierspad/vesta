<script lang="ts">
  import { t } from "./i18n";

  interface Props {
    isOpen: boolean;
    title: string;
    value: string;
    onclose: () => void;
    confirmText?: string;
    secondaryText?: string;
    onsecondary?: () => void;
    editable?: boolean;
    desc?: string;
    placeholder?: string;
    onsave?: (newValue: string) => boolean | Promise<boolean> | void | Promise<void>;
  }

  let {
    isOpen,
    title,
    value,
    onclose,
    confirmText = "OK",
    secondaryText,
    onsecondary,
    editable = false,
    desc,
    placeholder,
    onsave,
  }: Props = $props();

  let isEditing = $state(false);
  let editValue = $state("");

  // Keep editValue and isEditing state consistent when modal opens/changes
  $effect(() => {
    if (isOpen) {
      editValue = value;
      isEditing = false;
    }
  });

  async function handlePrimaryClick() {
    if (isEditing && onsave) {
      const result = await onsave(editValue);
      if (result === false) {
        return; // Validation failed, keep editing
      }
    }
    onclose();
  }

  function handleSecondaryClick() {
    if (editable) {
      if (isEditing) {
        // Cancel editing
        isEditing = false;
        editValue = value;
      } else {
        // Start editing
        isEditing = true;
        editValue = value;
      }
    } else {
      onsecondary?.();
    }
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-6"
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
      class="bg-gray-900 border border-gray-700 rounded-xl w-full max-w-2xl p-5 animate-fade-in"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="flex items-center justify-between mb-3">
        <h3 class="text-sm font-semibold text-gray-300">{title}</h3>
        <button
          type="button"
          onclick={onclose}
          class="text-gray-400 hover:text-white text-lg leading-none">✕</button
        >
      </div>

      {#if isEditing}
        <div class="flex flex-col">
          {#if desc}
            <p class="text-xs text-gray-400 mb-2">{desc}</p>
          {/if}
          <input
            type="text"
            bind:value={editValue}
            placeholder={placeholder || ""}
            class="input-modern w-full text-sm font-mono"
            onkeydown={(e) => {
              if (e.key === "Enter") handlePrimaryClick();
            }}
          />
        </div>
      {:else}
        <div class="bg-gray-800/80 rounded-lg p-3 border border-gray-700/50">
          <p class="text-sm text-white font-mono break-all select-all leading-relaxed">
            {value || "—"}
          </p>
        </div>
      {/if}

      {#if isEditing || editable || (secondaryText && onsecondary)}
        <div class="mt-4 flex justify-end gap-2">
          {#if editable}
            <button
              type="button"
              onclick={handleSecondaryClick}
              class="btn-secondary py-1.5 px-4 text-xs"
            >
              {isEditing ? t("common.cancel") : (secondaryText || "✏️ Edit")}
            </button>
          {:else if secondaryText && onsecondary}
            <button
              type="button"
              onclick={handleSecondaryClick}
              class="btn-secondary py-1.5 px-4 text-xs"
            >
              {secondaryText}
            </button>
          {/if}

          {#if isEditing}
            <button
              type="button"
              onclick={handlePrimaryClick}
              class="btn-primary py-1.5 px-4 text-xs"
            >
              {t("settings.modal.save")}
            </button>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

