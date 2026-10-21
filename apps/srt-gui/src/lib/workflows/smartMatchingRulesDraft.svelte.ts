import type { SmartMatchingRules } from "$lib/stores/smartMatchingStore.svelte";

export interface SmartMatchingRulesStore {
  enabled: boolean;
  rules: SmartMatchingRules;
  setEnabled(enabled: boolean): void;
  saveRules(rules: SmartMatchingRules): void;
}

export interface SmartMatchingRulesDraftMessages {
  invalidObject: () => string;
  invalidJson: () => string;
  saveSuccess: () => string;
  saveError: () => string;
}

export class SmartMatchingRulesDraft {
  value = $state("");
  error = $state<string | null>(null);

  constructor(
    private readonly store: SmartMatchingRulesStore,
    private readonly messages: SmartMatchingRulesDraftMessages,
    private readonly notifySuccess: (message: string) => void,
  ) {}

  private format(rules: SmartMatchingRules): string {
    return JSON.stringify(rules, null, 2);
  }

  loadFromStore(): void {
    this.value = this.format(this.store.rules);
    this.error = null;
  }

  private parseDraft(): unknown {
    const jsonWithoutComments = this.value.replace(
      /\\"|"(?:\\"|[^\"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g,
      (match, comment) => comment ? "" : match,
    );
    return JSON.parse(jsonWithoutComments);
  }

  validate(): string | null {
    try {
      const parsed = this.parseDraft();
      if (!parsed || typeof parsed !== "object") return this.messages.invalidObject();
      return null;
    } catch (error) {
      return error instanceof Error && error.message ? error.message : this.messages.invalidJson();
    }
  }

  toggle(): void {
    this.store.setEnabled(!this.store.enabled);
    if (this.store.enabled) {
      this.loadFromStore();
    }
  }

  save(): boolean {
    const validationError = this.validate();
    this.error = validationError;
    if (validationError) return false;

    try {
      this.store.saveRules(this.parseDraft() as SmartMatchingRules);
      this.notifySuccess(this.messages.saveSuccess());
      return true;
    } catch (error) {
      this.error = error instanceof Error && error.message ? error.message : this.messages.saveError();
      return false;
    }
  }
}
