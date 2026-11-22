import { openUrl } from "@tauri-apps/plugin-opener";

/** Route ordinary web links through the system browser in the desktop webview. */
export function installExternalLinks(root: Document, onError: (error: unknown) => void) {
  const handleClick = (event: MouseEvent) => {
    if (event.defaultPrevented || event.button !== 0) return;
    const anchor = event.composedPath().find(node => node instanceof HTMLAnchorElement);
    if (!(anchor instanceof HTMLAnchorElement) || anchor.hasAttribute("download")) return;
    const url = new URL(anchor.href, root.baseURI);
    if (!["https:", "http:", "mailto:"].includes(url.protocol)) return;
    if (url.origin === new URL(root.baseURI).origin) return;
    event.preventDefault();
    void openUrl(url.href).catch(onError);
  };
  root.addEventListener("click", handleClick);
  return () => root.removeEventListener("click", handleClick);
}
