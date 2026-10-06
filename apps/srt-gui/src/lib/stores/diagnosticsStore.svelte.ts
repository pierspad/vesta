import { invokeCommand } from "$lib/services/tauriClient";
import type { SystemDiagnostics } from "$lib/services/systemDiagnostics";

class DiagnosticsStore {
  data = $state<SystemDiagnostics | null>(null);
  loading = $state(false);
  error = $state(false);
  private loadedAt = 0;
  private pending: Promise<void> | null = null;

  refresh(force = false): Promise<void> {
    if (this.pending) return this.pending;
    if (!force && this.data && Date.now() - this.loadedAt < 60_000) return Promise.resolve();
    this.loading = true;
    this.error = false;
    this.pending = invokeCommand<SystemDiagnostics>("get_system_diagnostics")
      .then((data) => { this.data = data; this.loadedAt = Date.now(); })
      .catch((reason) => { this.error = true; console.error("System diagnostics", reason); })
      .finally(() => { this.loading = false; this.pending = null; });
    return this.pending;
  }
}
export const diagnosticsStore = new DiagnosticsStore();
