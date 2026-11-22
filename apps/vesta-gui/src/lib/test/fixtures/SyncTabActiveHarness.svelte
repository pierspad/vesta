<script lang="ts">
  import SyncTab from "../../tabs/SyncTab.svelte";

  let active = $state(true);
</script>

<button type="button" data-testid="deactivate-tab" onclick={() => (active = false)}>Deactivate</button>
<SyncTab {active} />
