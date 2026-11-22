<script lang="ts">
  import AudioClipsPanel from "$lib/panels/AudioClipsPanel.svelte";
  import { defaultMediaSettings } from "$lib/utils/mediaSettings";
  let settings = $state({ ...defaultMediaSettings });
  let panel: AudioClipsPanel;
  export async function previewEpisode(index: number) { await panel.previewEpisode(index); }
</script>
<button onclick={() => settings.generateAudio = !settings.generateAudio}>Toggle audio generation</button>
<AudioClipsPanel bind:this={panel} bind:settings hasAudio mediaType="audio" audioTracks={[]} audioTracksLoading={false}
  hintLoadMediaFirst="" episodes={[{id:1,mediaPath:"first.mp3",targetSubsPath:"first.srt",nativeSubsPath:""}]} onTrackPicked={() => {}} />
