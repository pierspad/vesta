<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  interface Props {
    show?: boolean;
    title?: string;
    message?: string;
    confirmText?: string;
    cancelText?: string;
    variant?: 'danger' | 'warning' | 'info' | 'success';
  }

  let {
    show = false,
    title = '',
    message = '',
    confirmText = 'Conferma',
    cancelText = 'Annulla',
    variant = 'danger'
  }: Props = $props();

  const dispatch = createEventDispatcher<{
    confirm: void;
    cancel: void;
  }>();

  function handleCancel() {
    dispatch('cancel');
  }

  function handleConfirm() {
    dispatch('confirm');
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!show) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      handleCancel();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      handleConfirm();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if show}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="confirm-modal-overlay"
    onclick={handleCancel}
    role="dialog"
    aria-modal="true"
    aria-labelledby="confirm-modal-title"
    tabindex="-1"
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="confirm-modal-card" onclick={(e) => e.stopPropagation()} role="document">
      <div class="confirm-modal-body">
        <div class="confirm-modal-icon-wrap {variant}">
          {#if variant === 'danger'}
            <!-- Alert Circle SVG -->
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
          {:else if variant === 'warning'}
            <!-- Alert Triangle SVG -->
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
          {:else if variant === 'success'}
            <!-- Check Circle SVG -->
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          {:else}
            <!-- Info SVG -->
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          {/if}
        </div>
        <div class="confirm-modal-text">
          <h3 id="confirm-modal-title" class="text-base font-bold text-white leading-tight m-0">{title}</h3>
          <p class="text-xs text-gray-400 leading-normal mt-1 m-0">{message}</p>
        </div>
      </div>
      <div class="confirm-modal-actions flex justify-end gap-3">
        <button class="confirm-btn-cancel px-4 py-2 rounded-lg text-xs font-semibold cursor-pointer border border-white/10 bg-white/5 text-gray-300 hover:bg-white/10 transition-colors" type="button" onclick={handleCancel}>
          {cancelText}
        </button>
        <button class="confirm-btn-action {variant} px-4 py-2 rounded-lg text-xs font-semibold cursor-pointer border-none text-white transition-colors" type="button" onclick={handleConfirm}>
          {confirmText}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .confirm-modal-overlay {
    position: fixed;
    inset: 0;
    z-index: 9999;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(8px);
  }

  .confirm-modal-card {
    background: #1e1e2e;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    padding: 24px;
    max-width: 440px;
    width: 100%;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.55);
    display: flex;
    flex-direction: column;
    gap: 20px;
    animation: confirmModalScale 0.15s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  @keyframes confirmModalScale {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .confirm-modal-body {
    display: flex;
    align-items: flex-start;
    gap: 16px;
  }

  .confirm-modal-icon-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  /* Variant colors */
  .confirm-modal-icon-wrap.danger {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.15);
  }
  .confirm-modal-icon-wrap.warning {
    color: #f59e0b;
    background: rgba(245, 158, 11, 0.15);
  }
  .confirm-modal-icon-wrap.success {
    color: #22c55e;
    background: rgba(34, 197, 94, 0.15);
  }
  .confirm-modal-icon-wrap.info {
    color: #3b82f6;
    background: rgba(59, 130, 246, 0.15);
  }

  .confirm-modal-text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .confirm-btn-action.danger {
    background: #ef4444;
  }
  .confirm-btn-action.danger:hover {
    background: #dc2626;
  }

  .confirm-btn-action.warning {
    background: #f59e0b;
  }
  .confirm-btn-action.warning:hover {
    background: #d97706;
  }

  .confirm-btn-action.success {
    background: #22c55e;
  }
  .confirm-btn-action.success:hover {
    background: #16a34a;
  }

  .confirm-btn-action.info {
    background: #3b82f6;
  }
  .confirm-btn-action.info:hover {
    background: #2563eb;
  }
</style>
