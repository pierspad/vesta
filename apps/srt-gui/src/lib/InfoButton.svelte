<script lang="ts">
  interface Props {
    onclick: () => void;
    title?: string;
    class?: string;
  }

  let { onclick, title = "Info", class: className = "ml-auto text-gray-500 hover:text-cyan-300 transition-colors" }: Props = $props();
</script>

<button
  type="button"
  {onclick}
  class={className}
  {title}
>
  <svg
    class="w-3.5 h-3.5"
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      stroke-linecap="round"
      stroke-linejoin="round"
      stroke-width="2"
      d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
    />
  </svg>
</button>
