<script lang="ts">
  import SnapshotsPanel from "$lib/panels/SnapshotsPanel.svelte";
  import { defaultMediaSettings } from "$lib/utils/mediaSettings";
  let settings = $state({ ...defaultMediaSettings, snapshotWidth: 640, snapshotHeight: 360 });
</script>
<SnapshotsPanel bind:settings hasVideo={true} effectiveExportFormat="apkg" hintLoadVideoFirst="Load video" />
