<script lang="ts">
  let { onTabChange, settingsSection = $bindable("overview") } = $props<{
    onTabChange: (tab: string) => void;
    settingsSection?: string;
  }>();
</script>

<nav>
  <button data-testid="switch-flashcards" onclick={() => onTabChange("flashcards")}>Flashcards</button>
  <button data-testid="switch-translate" onclick={() => onTabChange("translate")}>Translate</button>
</nav>
