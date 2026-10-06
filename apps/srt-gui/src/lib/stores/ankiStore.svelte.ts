import { invokeCommand as invoke } from "$lib/services/tauriClient";
import * as vestaConfig from "$lib/config/vestaConfig";

class AnkiStore {
  url = $state("http://127.0.0.1:8765");
  status = $state<"unknown" | "checking" | "online" | "offline">("unknown");
  version = $state<number | null>(null);
  decks = $state<string[]>([]);
  autoCardFont = $state<boolean>(true);
  embedCardFont = $state<boolean>(true);

  constructor() {
    this.url = vestaConfig.getItem("vesta-ankiconnect-url") || "http://127.0.0.1:8765";
    const savedAutoFont = vestaConfig.getItem("vesta-auto-card-font");
    this.autoCardFont = savedAutoFont !== "false";
    const savedEmbedFont = vestaConfig.getItem("vesta-embed-card-font");
    this.embedCardFont = savedEmbedFont !== "false";
  }

  setUrl(newUrl: string) {
    this.url = newUrl;
    try {
      vestaConfig.setItem("vesta-ankiconnect-url", newUrl);
    } catch {}
  }

  setAutoCardFont(val: boolean) {
    this.autoCardFont = val;
    try {
      vestaConfig.setItem("vesta-auto-card-font", String(val));
    } catch {}
  }

  setEmbedCardFont(val: boolean) {
    this.embedCardFont = val;
    try {
      vestaConfig.setItem("vesta-embed-card-font", String(val));
    } catch {}
  }

  async checkConnection(): Promise<boolean> {
    this.status = "checking";
    try {
      this.version = await invoke<number>("ankiconnect_ping", { url: this.url });
      this.decks = await invoke<string[]>("ankiconnect_deck_names", { url: this.url });
      this.status = "online";
      return true;
    } catch (err) {
      this.status = "offline";
      this.version = null;
      this.decks = [];
      return false;
    }
  }
}

export const ankiStore = new AnkiStore();
