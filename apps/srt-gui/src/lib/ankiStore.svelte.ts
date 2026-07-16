import { invoke } from "@tauri-apps/api/core";
import * as vestaConfig from "./vestaConfig";

class AnkiStore {
  url = $state("http://127.0.0.1:8765");
  status = $state<"unknown" | "checking" | "online" | "offline">("unknown");
  version = $state<number | null>(null);
  decks = $state<string[]>([]);

  constructor() {
    this.url = vestaConfig.getItem("vesta-ankiconnect-url") || "http://127.0.0.1:8765";
  }

  setUrl(newUrl: string) {
    this.url = newUrl;
    try {
      vestaConfig.setItem("vesta-ankiconnect-url", newUrl);
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
