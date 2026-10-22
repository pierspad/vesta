<script lang="ts">
  import { tick } from "svelte";
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  const baselines = import.meta.glob("./CodeEditorScrollBaseline.svelte", { eager: true, import: "default" });
  const Baseline = Object.values(baselines)[0] as typeof CodeEditor | undefined;
  let Editor = $state<typeof CodeEditor>(CodeEditor);
  let value = $state("");
  let running = $state(false);
  let status = $state("Ready");
  let host: HTMLDivElement;
  let results = $state<Record<string, unknown>[]>([]);
  const frame = () => new Promise<number>(resolve => requestAnimationFrame(resolve));
  function percentile(values: number[], fraction: number) { return [...values].sort((a,b)=>a-b)[Math.min(values.length-1, Math.floor(values.length*fraction))]; }
  async function sample(label: string, lines: number, repeat: number) {
    status = `${label}: ${lines} lines, sample ${repeat}`;
    await tick(); await frame(); await frame();
    const input = host.querySelector("textarea")!;
    input.focus(); input.scrollTop = 0; await frame(); await frame();
    const maxScroll = input.scrollHeight - input.clientHeight;
    let mutations = 0;
    const observer = new MutationObserver(records => mutations += records.length);
    observer.observe(host, {subtree:true, attributes:true, attributeFilter:["style"]});
    const intervals: number[] = [];
    let previous = await frame();
    for (let i=0;i<120;i++) {
      input.scrollTop = maxScroll * (0.5 - Math.cos(i / 119 * Math.PI * 2) / 2);
      const now = await frame(); intervals.push(now-previous); previous=now;
    }
    await frame(); await frame(); observer.disconnect();
    results.push({variant:label, lines, repeat, medianFrameMs:percentile(intervals,0.5), p95FrameMs:percentile(intervals,0.95), framesOver25Ms:intervals.filter(v=>v>25).length, styleMutations:mutations, nodes:host.querySelectorAll("*").length});
  }
  async function run() {
    if (running) return;
    running=true; results=[];
    try {
      for (const lines of [200,2000]) {
        value=Array.from({length:lines},(_,i)=>`Rule ${i}: Review **{Expression}** and {Meaning}. Preserve punctuation, subtitle timing and the original language of each sentence.`).join("\n");
        // Alternate order so warm-up and temperature do not always favor one variant.
        for (let repeat=1;repeat<=3;repeat++) {
          const variants: [string, typeof CodeEditor][] = Baseline ? (repeat%2 ? [["baseline",Baseline],["current",CodeEditor]] : [["current",CodeEditor],["baseline",Baseline]]) : [["current",CodeEditor]];
          for (const [label,component] of variants) { Editor=component; await sample(label,lines,repeat); }
        }
      }
      status="Complete";
    } finally { running=false; }
  }
</script>
<div class="mx-auto max-w-5xl p-6 text-gray-200 space-y-4">
  <h1 class="text-xl font-semibold">Editor scroll benchmark</h1>
  <p>Wrapped prompt text, 120 scrolling frames, three alternating samples per workload. No media or cloud calls. Browser frame timing includes compositor and display scheduling; this is not a native WebKit CPU profile.</p>
  <button class="btn-secondary" disabled={running} onclick={run}>Run benchmark</button>
  <p role="status">{status}</p>
  <div class="glass-card p-4" bind:this={host}>{#key Editor}<Editor bind:value language="prompt" heightClass="h-80"/>{/key}</div>
  <pre class="overflow-auto whitespace-pre-wrap text-xs" data-testid="benchmark-results">{JSON.stringify(results,null,2)}</pre>
</div>
