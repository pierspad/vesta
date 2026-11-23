import type { PluginManifest } from "$lib/services/pluginClient";

export function getPluginImageSrc(manifest?: PluginManifest, iconUrl?: string): string | null {
  if (iconUrl) return iconUrl;
  const raw = manifest?.icon || manifest?.tab?.icon;
  if (raw && (raw.startsWith("data:image/") || raw.startsWith("https://") || raw.startsWith("http://") || raw.startsWith("/"))) {
    return raw;
  }
  return null;
}

export interface ResolvedHintAndLink {
  text: string;
  link?: { label: string; url: string };
}

export function resolveHintAndLink(
  hint?: string | { text?: string; link?: { label: string; url: string } },
  helpText?: string,
  link?: { label: string; url: string }
): ResolvedHintAndLink {
  let rawText = "";
  let extractedLink = link;

  if (typeof hint === "string") {
    rawText = hint;
  } else if (hint && typeof hint === "object") {
    rawText = hint.text ?? "";
    if (!extractedLink && hint.link) {
      extractedLink = hint.link;
    }
  } else if (helpText) {
    rawText = helpText;
  }

  const mdMatch = rawText.match(/\[([^\]]+)\]\((https?:\/\/[^\)]+)\)/);
  if (mdMatch) {
    if (!extractedLink) {
      extractedLink = { label: mdMatch[1], url: mdMatch[2] };
    }
    rawText = rawText.replace(mdMatch[0], "").trim();
  }

  if (extractedLink && extractedLink.label) {
    rawText = rawText.replace(`[${extractedLink.label}]`, "").trim();
  }

  return { text: rawText, link: extractedLink };
}

export function formatPermission(perm: string, t: (key: string) => string): string {
  if (perm === "network") return t("plugins.permNetwork");
  if (perm === "filesystem:downloads") return t("plugins.permDownloads");
  if (perm === "vesta:cli") return t("plugins.permVestaCli");
  if (perm.startsWith("filesystem:")) return perm.replace("filesystem:", "Filesystem: ");
  return perm;
}

export function getPermissionScopes(permissions?: unknown): string[] {
  if (!permissions) return [];
  if (Array.isArray(permissions)) {
    return permissions.filter(
      (p): p is string =>
        typeof p === "string" &&
        !p.startsWith("sharedCredential:") &&
        !p.startsWith("shared_credential:") &&
        !p.startsWith("sharedCredentials:") &&
        !p.startsWith("shared:")
    );
  }
  if (typeof permissions === "object" && permissions !== null && "scopes" in permissions && Array.isArray((permissions as { scopes: unknown }).scopes)) {
    return (permissions as { scopes: string[] }).scopes;
  }
  return [];
}

export function formatPluginDate(iso?: string | null): string {
  if (!iso) return "—";
  try {
    const d = new Date(iso);
    return isNaN(d.getTime())
      ? iso
      : d.toLocaleDateString(undefined, {
          year: "numeric",
          month: "short",
          day: "numeric",
          hour: "2-digit",
          minute: "2-digit",
        });
  } catch {
    return iso;
  }
}

export const SDK_GUIDE_MARKDOWN = `# Vesta Plugin System & SDK Contract (v1)

Plugins in Vesta are **100% Vesta-agnostic** child processes communicating over **JSON-RPC 2.0 via standard input/output (stdio)**.

As long as Vesta honors this v1 contract, Vesta's internal GUI and Rust core can evolve freely without breaking third-party plugins.

## 1. Plugin Manifest & Visual Customization
Every plugin declares a \`vesta-plugin.json\` manifest with \`id\`, \`name\`, \`version\`, \`apiVersion: 1\`, and process execution \`entry\`.
- \`icon\`: Top-level image or icon specification. Supports named monochrome icons (\`"puzzle"\`, \`"film"\`, \`"search"\`), relative image files within the plugin repository (\`"icon.svg"\`, \`"icon.png"\`, \`"icon.webp"\`, \`"assets/logo.svg"\`, max 512 KB), or \`data:image/...\` / \`https://\` URIs.
- \`tab\`: Visual metadata for the plugin header and card (\`{ title: string, icon?: string }\`).

## 2. Host Context & Properties Provided by Vesta
On \`initialize\`, \`render\`, and \`event\`, Vesta automatically passes:
- \`apiVersion\`: \`1\`
- \`dataDir\`: Dedicated persistent directory for the plugin (\`<app_data>/plugins/installed/<id>/data\`)
- \`locale\`: Active Vesta UI language code (e.g. \`"en"\`, \`"it"\`, \`"ja"\`)
- \`expertMode\`: \`boolean\` indicating whether the user enabled **Expert Mode** in Vesta
- \`settings\`: Dictionary of saved manifest settings
- \`values\`: Snapshot of all interactive widget values on the active screen (on \`event\`)

## 3. Plugin credentials
Store credentials in the plugin's own settings or data directory. Vesta does not forward credentials between plugins.

## 4. Declarative Widget Catalog
Every widget supports \`id\`, \`hidden\`, \`expertOnly\` (auto-hidden when Expert Mode is off), \`tooltip\` (native Vesta hover tooltip), \`flex\`, and \`width\`.

- **Layout**: \`column\` (\`gap\`, \`children\`), \`row\` (\`gap\`, \`align\`: \`"start" | "center" | "end" | "between"\`, \`separator\`, \`children\`), \`divider\`, \`heading\` (\`level\`, \`icon\`), \`text\` (\`variant\`, \`size\`)
- **Inputs**:
  - \`textInput\` / \`passwordInput\`: \`label\`, \`labelIcon\`, \`labelTooltip\`, \`placeholder\`, \`defaultValue\`, \`icon\` (\`"search"\`, \`"tmdb"\`, \`"imdb"\`, etc.), \`submitAction\` (button ID triggered when pressing **Enter**), \`emptyWarning\`, \`hint\`, \`link\`
  - \`textArea\`: Resizable multiline input (\`label\`, \`placeholder\`, \`defaultValue\`, \`rows\`, \`maxLength\`, \`disabled\`, \`readOnly\`)
  - \`languageSelect\`: Unified Vesta language selector with bundled SVG flags (\`scope\`: \`"all"\` for 58 world languages or \`"core"\` for 33 study languages)
  - \`pathInput\`: Native OS file/folder selector (\`pickerType\`: \`"file" | "folder"\`, \`filters\`, \`drop\`, \`dropHint\`)
  - \`select\`, \`segmented\`, \`numberInput\`, \`checkbox\`
- **Actions & Display**:
  - \`button\`: \`label\`, \`variant\` (\`"primary" | "secondary" | "danger" | "settings" | "accent"\`), \`status\`, \`placement\`, \`icon\`, \`disabled\`, \`tooltip\`, \`disabledTooltip\`, \`disabledFocus\`, \`requiresAny\`
  - \`list\`: \`items\` with \`title\`, \`subtitle\`, \`badge\`, \`language\` (ISO code automatically rendered by Vesta as **Flag SVG + Localized Language Name** in Vesta's active language), \`meta\`, \`image\`, \`primaryAction\`, and \`actions\`
  - \`fileOutput\`, \`progress\`, \`table\`, \`image\`, \`link\` / \`externalLink\`

## 5. Host Notifications & Integrations (Plugin → Vesta)
- **Host Snackbar / Toast**: Return \`snackbar: { message: "Saved!", type: "success" | "error" | "info" | "warning" }\` in \`render\` or \`event\` response to trigger native Vesta toast notification.
- \`ui.update\`: Push live \`root\` or incremental \`patch\` (\`values\`, \`setProps\`, \`visibility\`, \`insert\`, \`remove\`, \`move\`)
- \`ui.notify\`: Show native toast (\`level\`: \`"info" | "success" | "warn" | "error"\`, \`msg\`)
- \`ui.progress\`: Update progress indicator (\`id\`, \`value\`, \`label\`)
- \`vesta.openInFlashcards\`: Hand off \`videoPath\` and \`subtitlePath\` directly to the Flashcards tab
- \`vesta.cli\`: Execute headless Vesta CLI workflows (\`flashcards\`, \`translate\`, \`transcribe\`, \`sync\`, \`autosync\`, \`refine\`, \`extract\`, \`ocr\`) when the plugin declares the \`"vesta:cli"\` permission scope

## 6. MCP Server Support (\`vesta-mcp\`)
\`vesta-mcp\` exposes \`list_plugins\` (safe public metadata, zero secrets) and \`call_plugin\` (JSON-RPC render/event bridge) for AI agents.`;
