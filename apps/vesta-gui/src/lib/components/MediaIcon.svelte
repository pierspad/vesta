<script lang="ts">
  let { kind, class: className = "w-3.5 h-3.5 inline-block shrink-0" }: { kind: "audio" | "snapshot" | "package" | "video" | "folder" | "file"; class?: string } = $props();
  const paths = {
    audio: "M9 18V5l12-2v13M9 8l12-2M9 18a3 3 0 1 1-3-3c1.66 0 3 1.34 3 3zm12-2a3 3 0 1 1-3-3c1.66 0 3 1.34 3 3z",
    snapshot: "M4 4h16v16H4zM4 16l5-5 4 4 3-3 4 4M15 8h.01",
    package: "m3 7 9-4 9 4-9 4-9-4zm0 0v10l9 4 9-4V7M12 11v10M7.5 5l9 4",
    video: "M3 5h13v14H3zM16 10l5-3v10l-5-3",
    folder: "M3 7v12h18V7H12l-2-3H3v3z",
    file: "M6 3h8l4 4v14H6zM14 3v5h4M9 12h6M9 16h6",
  };
</script>
<svg class={className} aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d={paths[kind]} /></svg>
