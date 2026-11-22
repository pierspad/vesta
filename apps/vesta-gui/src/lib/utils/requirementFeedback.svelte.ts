/** Reuses the existing brief requirement pulse without adding persistent UI text. */
export class RequirementFeedback<Panel extends string> {
  highlighted = $state<Set<Panel>>(new Set());
  private timeout: ReturnType<typeof setTimeout> | null = null;
  pulse(panels: Panel[]) {
    this.clear();
    this.highlighted = new Set(panels);
    this.timeout = setTimeout(() => this.clear(), 1800);
  }
  classFor(panel: Panel) { return this.highlighted.has(panel) ? "requirement-pulse" : ""; }
  clear() {
    if (this.timeout !== null) clearTimeout(this.timeout);
    this.timeout = null;
    this.highlighted = new Set();
  }
}
