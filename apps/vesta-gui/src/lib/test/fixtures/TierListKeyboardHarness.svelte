<script lang="ts">
  import TierList from "../../components/TierList.svelte";

  export let onAction = (_action: string) => {};
  const tiers = [{ id: "first" }, { id: "second" }];
</script>

{#snippet tierBody(tier: { id: string })}
  <span data-testid="body">{tier.id}</span>
{/snippet}

<TierList
  {tiers}
  emptyMessage="No tiers"
  priorityLabel={(index) => `Priority ${index + 1}`}
  moveUpLabel="Move up"
  moveDownLabel="Move down"
  removeTierLabel="Remove"
  onRemoveTier={() => onAction("remove")}
  onMoveTier={(_index, direction) => onAction(direction === -1 ? "up" : "down")}
  tierBody={tierBody}
/>
