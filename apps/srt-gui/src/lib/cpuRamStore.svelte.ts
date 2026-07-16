import { invoke } from "@tauri-apps/api/core";
import * as vestaConfig from "./vestaConfig";

/** CPU-core / RAM-limit resource card in Settings -> Overview. Self-contained
 * to this one card (verified: every reference outside the card's markup is
 * this store's own init logic, not another section) -- EXCEPT `cpuCores`
 * changes are also broadcast as a `vesta-cpu-cores-changed` window event,
 * which FlashcardsTab.svelte listens for independently. `init()` mirrors
 * whisperModelsStore's pattern: called once from SettingsTab.svelte's
 * app-lifetime onMount rather than this card's own (section-scoped) mount,
 * since it just reads system info once and there's no harm/benefit either
 * way -- kept consistent with the other overview cards for the same reason
 * documented in [[vesta-settings-refactor]]. */
class CpuRamStore {
  systemCpuCount = $state(4);
  cpuCores = $state(2);
  minCpuCores = $derived(1);
  maxCpuCores = $derived(Math.max(1, this.systemCpuCount - 1));

  systemTotalMemoryMb = $state(4096);
  minRamMb = $derived(256);
  maxRamMb = $derived(Math.max(512, this.systemTotalMemoryMb - 512));
  /** 0 means "no limit" (use OS default). */
  ramLimitMb = $state(
    (() => {
      const stored = vestaConfig.getItem("vesta_memory_limit_mb");
      return stored ? parseInt(stored) : 0;
    })(),
  );

  ramTicksMb = $derived.by(() => {
    const maxGb = this.maxRamMb / 1024;
    const rawStep = maxGb / 6;
    const niceStep =
      rawStep <= 1 ? 1
      : rawStep <= 2 ? 2
      : rawStep <= 4 ? 4
      : rawStep <= 8 ? 8
      : rawStep <= 16 ? 16
      : 32;
    const ticks: number[] = [0];
    for (let gb = niceStep; gb * 1024 <= this.maxRamMb; gb += niceStep) {
      ticks.push(Math.round(gb * 1024));
    }
    return ticks;
  });

  cpuTickStep = $derived(
    (this.maxCpuCores - this.minCpuCores) <= 8 ? 1
    : (this.maxCpuCores - this.minCpuCores) <= 16 ? 2
    : (this.maxCpuCores - this.minCpuCores) <= 32 ? 4
    : 8,
  );

  cpuPresets = $derived([
    { id: "eco", threads: this.minCpuCores },
    { id: "balanced", threads: this.minCpuCores + Math.ceil((this.maxCpuCores - this.minCpuCores) / 3) },
    { id: "performance", threads: this.minCpuCores + Math.ceil(((this.maxCpuCores - this.minCpuCores) * 2) / 3) },
    { id: "full", threads: this.maxCpuCores },
  ] as const);

  activeCpuPreset = $derived(this.cpuPresets.find((p) => p.threads === this.cpuCores)?.id ?? null);

  setCores(cores: number) {
    this.cpuCores = cores;
    vestaConfig.setItem("vesta_cpu_cores", this.cpuCores.toString());
    window.dispatchEvent(new CustomEvent("vesta-cpu-cores-changed", { detail: this.cpuCores }));
  }

  setCpuPreset(presetId: string) {
    const preset = this.cpuPresets.find((p) => p.id === presetId);
    if (preset) this.setCores(preset.threads);
  }

  setRamLimitMb(raw: number) {
    // 0 = no limit
    this.ramLimitMb = raw < this.minRamMb ? 0 : raw;
    if (this.ramLimitMb === 0) {
      vestaConfig.removeItem("vesta_memory_limit_mb");
    } else {
      vestaConfig.setItem("vesta_memory_limit_mb", this.ramLimitMb.toString());
    }
  }

  /** Called once from SettingsTab.svelte's app-lifetime onMount. */
  async init() {
    try {
      const count = await invoke<number>("flashcard_get_cpu_count");
      this.systemCpuCount = count;
      const savedCores = vestaConfig.getItem("vesta_cpu_cores");
      if (savedCores) {
        const parsed = parseInt(savedCores);
        this.cpuCores = Math.min(Math.max(parsed, this.minCpuCores), Math.max(1, this.systemCpuCount - 1));
      } else {
        this.cpuCores = Math.max(1, this.systemCpuCount - 1);
      }
    } catch {
      this.systemCpuCount = 4;
      const savedCores = vestaConfig.getItem("vesta_cpu_cores");
      this.cpuCores = savedCores ? parseInt(savedCores) : Math.max(1, this.systemCpuCount - 1);
    }

    try {
      const mb = await invoke<number>("flashcard_get_total_memory_mb");
      this.systemTotalMemoryMb = mb;
      // Validate stored ramLimitMb against new bounds
      if (this.ramLimitMb > 0) {
        this.ramLimitMb = Math.min(Math.max(this.ramLimitMb, this.minRamMb), Math.max(512, mb - 512));
      }
    } catch {
      this.systemTotalMemoryMb = 4096;
    }
  }
}

export const cpuRamStore = new CpuRamStore();
