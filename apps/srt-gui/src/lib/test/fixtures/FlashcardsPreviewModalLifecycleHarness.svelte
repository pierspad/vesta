<script lang="ts">
  import FlashcardsPreviewModal from "../../modals/FlashcardsPreviewModal.svelte";

  let active = $state(true);
</script>

<button type="button" data-testid="deactivate-tab" onclick={() => (active = false)}>Deactivate tab</button>
<span data-testid="owner-state">{active ? "active" : "inactive"}</span>
<FlashcardsPreviewModal mediaPath={null} mediaType="none" nativeSubsPath="" {active} />
