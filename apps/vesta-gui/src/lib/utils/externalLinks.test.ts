// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn().mockResolvedValue(undefined) }));
import { openUrl } from "@tauri-apps/plugin-opener";
import { installExternalLinks } from "./externalLinks";
let cleanup = () => {};
const preventNavigation = (event: Event) => event.preventDefault();
beforeEach(() => window.addEventListener("click", preventNavigation));
afterEach(() => { window.removeEventListener("click", preventNavigation); cleanup(); document.body.replaceChildren(); vi.clearAllMocks(); });
it("opens nested external link clicks in the system browser", () => {
  cleanup = installExternalLinks(document, vi.fn());
  document.body.innerHTML = '<a href="https://example.com/docs" target="_blank"><span>Docs</span></a>';
  const event = new MouseEvent("click", { bubbles: true, cancelable: true });
  document.querySelector("span")!.dispatchEvent(event);
  expect(event.defaultPrevented).toBe(true);
  expect(openUrl).toHaveBeenCalledWith("https://example.com/docs");
});
it("preserves in-app, download and already handled clicks", () => {
  cleanup = installExternalLinks(document, vi.fn());
  for (const attributes of ['href="#panel"', 'href="javascript:void(0)"', 'href="https://example.com/file" download']) {
    document.body.innerHTML = `<a ${attributes}>Link</a>`;
    document.querySelector("a")!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  }
  document.body.innerHTML = '<a href="https://example.com">Link</a>';
  const event = new MouseEvent("click", { bubbles: true, cancelable: true });
  event.preventDefault(); document.querySelector("a")!.dispatchEvent(event);
  expect(openUrl).not.toHaveBeenCalled();
});
it("reports native opening failures", async () => {
  const error = new Error("no browser"); vi.mocked(openUrl).mockRejectedValueOnce(error);
  const onError = vi.fn(); cleanup = installExternalLinks(document, onError);
  document.body.innerHTML = '<a href="https://failure.example.org">Link</a>';
  document.querySelector("a")!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(onError).toHaveBeenCalledWith(error));
  expect(onError).toHaveBeenCalledWith(error);
});
