<script lang="ts">
  import AlignTab from "$lib/tabs/AlignTab.svelte";
  let active = $state(true);
  export function setActive(value: boolean) { active = value; }
</script>

<AlignTab {active} />
