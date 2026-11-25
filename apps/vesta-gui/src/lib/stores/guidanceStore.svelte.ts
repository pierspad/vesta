import { TIERS_UPDATED_EVENT, loadTiers, tiersHaveUsableEntries } from "$lib/config/translationTiers";
import { TRANSCRIBE_TIERS_UPDATED_EVENT } from "$lib/config/transcribeTiers";
import { ITT_TIERS_UPDATED_EVENT } from "$lib/config/ittTiers";

export type GuidanceSection = "llm" | "whisper" | "itt";

export interface SettingsGuidance {
  sourceTab: string;
  section: GuidanceSection;
  targetId: string;
}

export type GuidancePhase = "sidebar-settings" | "settings-section";

const PULSE_DURATION_MS = 1800;
const PAUSE_DURATION_MS = 2200;
const MAX_PULSE_CYCLES = 3;

export class GuidanceStore {
  active = $state<SettingsGuidance | null>(null);
  phase = $state<GuidancePhase | null>(null);
  pulseOn = $state(false);

  private hasEnteredSettings = false;
  private enteredSection: string | null = null;
  private cycleCount = 0;
  private pulseTimer: ReturnType<typeof setTimeout> | null = null;
  private pauseTimer: ReturnType<typeof setTimeout> | null = null;
  private listenersAttached = false;

  get isSettingsButtonPulsing(): boolean {
    return this.pulseOn && this.phase === "sidebar-settings";
  }

  isSectionPulsing(section: string): boolean {
    return this.pulseOn && this.phase === "settings-section" && this.active?.section === section;
  }

  guideToSettings(target: SettingsGuidance) {
    this.clearTimers();
    this.active = target;
    this.phase = "sidebar-settings";
    this.hasEnteredSettings = false;
    this.enteredSection = null;
    this.ensureListeners();
    this.startPulseCycles();
  }

  /**
   * Advances or cancels the active step-by-step guidance based on navigation.
   * Cancels immediately if the user navigates off-path or backs out halfway.
   */
  syncNavigation(activeTab: string, settingsSection: string, onArrive: (targetId: string) => void) {
    const current = this.active;
    if (!current) return;

    if (activeTab !== "settings") {
      if (this.hasEnteredSettings || activeTab !== current.sourceTab) {
        this.clear();
      }
      return;
    }

    if (settingsSection === current.section) {
      const targetId = current.targetId;
      this.clear();
      onArrive(targetId);
      return;
    }

    if (!this.hasEnteredSettings) {
      this.hasEnteredSettings = true;
      this.enteredSection = settingsSection;
      this.phase = "settings-section";
      this.startPulseCycles();
      return;
    }

    if (settingsSection !== this.enteredSection) {
      this.clear();
    }
  }

  clear() {
    this.clearTimers();
    this.active = null;
    this.phase = null;
    this.pulseOn = false;
    this.hasEnteredSettings = false;
    this.enteredSection = null;
    this.cycleCount = 0;
  }

  private startPulseCycles() {
    this.clearTimers();
    this.cycleCount = 0;
    this.triggerSingleCycle();
  }

  private triggerSingleCycle() {
    if (!this.active) return;
    this.pulseOn = true;
    this.pulseTimer = setTimeout(() => {
      this.pulseTimer = null;
      this.pulseOn = false;
      this.cycleCount += 1;
      if (this.cycleCount >= MAX_PULSE_CYCLES) {
        this.clear();
        return;
      }
      this.pauseTimer = setTimeout(() => {
        this.pauseTimer = null;
        this.triggerSingleCycle();
      }, PAUSE_DURATION_MS);
    }, PULSE_DURATION_MS);
  }

  private clearTimers() {
    if (this.pulseTimer !== null) {
      clearTimeout(this.pulseTimer);
      this.pulseTimer = null;
    }
    if (this.pauseTimer !== null) {
      clearTimeout(this.pauseTimer);
      this.pauseTimer = null;
    }
  }

  private ensureListeners() {
    if (this.listenersAttached || typeof window === "undefined") return;
    this.listenersAttached = true;

    window.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && this.active) {
        this.clear();
      }
    });

    window.addEventListener(TIERS_UPDATED_EVENT, () => {
      if (this.active?.section === "llm" && tiersHaveUsableEntries(loadTiers())) {
        this.clear();
      }
    });

    window.addEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, () => {
      if (this.active?.section === "whisper") {
        this.clear();
      }
    });

    window.addEventListener(ITT_TIERS_UPDATED_EVENT, () => {
      if (this.active?.section === "itt") {
        this.clear();
      }
    });

    window.addEventListener("vesta-whisper-models-updated", () => {
      if (this.active?.section === "whisper") {
        this.clear();
      }
    });

    window.addEventListener("vesta-ocr-models-updated", () => {
      if (this.active?.section === "itt") {
        this.clear();
      }
    });
  }
}

export const guidanceStore = new GuidanceStore();
