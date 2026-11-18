<script lang="ts">
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";

  interface Props {
    variant?: "danger" | "warning" | "info" | "success";
    conditionalMount?: boolean;
    onconfirm?: () => void;
    oncancel?: () => void;
  }

  let {
    variant = "danger",
    conditionalMount = false,
    onconfirm,
    oncancel,
  }: Props = $props();

  let open = $state(false);
</script>

<button type="button" data-testid="trigger" onclick={() => (open = true)}>Open</button>
<button type="button" data-testid="outside">Outside</button>

{#if !conditionalMount || open}
  <ConfirmDialog
    show={open}
    title="Title"
    message="Message"
    {variant}
    on:confirm={() => {
      open = false;
      onconfirm?.();
    }}
    on:cancel={() => {
      open = false;
      oncancel?.();
    }}
  />
{/if}
