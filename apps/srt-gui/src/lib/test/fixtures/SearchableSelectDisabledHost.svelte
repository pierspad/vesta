<script lang="ts">
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  let disabled = $state(false);
  let value = $state("one");
  const options = [{ value: "one", label: "One" }, { value: "two", label: "Two" }];
</script>

<SearchableSelect {options} {value} {disabled} portal onchange={(next) => value = next} />
<button type="button" onclick={() => disabled = true}>Disable selector</button>
