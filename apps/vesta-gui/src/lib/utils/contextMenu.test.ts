// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { contextMenu } from "./contextMenu";
afterEach(() => { document.body.replaceChildren(); vi.restoreAllMocks(); });
it("fits measured menus, skips disabled items and restores keyboard focus", () => {
 const input = document.createElement("input"); document.body.append(input); input.focus();
 const menu = document.createElement("div");
 menu.innerHTML = '<button>Copy</button><button disabled>Cut</button><button>Paste</button>';
 document.body.append(menu);
 vi.spyOn(menu, "getBoundingClientRect").mockReturnValue({ width: 250, height: 160 } as DOMRect);
 const close = vi.fn(); const action = contextMenu(menu, { x: window.innerWidth, y: window.innerHeight, close });
 expect(menu.style.left).toBe(`${window.innerWidth - 258}px`);
 expect(menu.style.top).toBe(`${window.innerHeight - 168}px`);
 expect(document.activeElement?.textContent).toBe("Copy");
 const press = (key: string) => document.activeElement!.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
 press("ArrowDown"); expect(document.activeElement?.textContent).toBe("Paste");
 press("ArrowDown"); expect(document.activeElement?.textContent).toBe("Copy");
 press("End"); expect(document.activeElement?.textContent).toBe("Paste");
 press("Escape"); expect(close).toHaveBeenCalledOnce();
 action.update({x:-30,y:-20,close}); expect(menu.style.left).toBe("8px"); expect(menu.style.top).toBe("8px");
 action.destroy(); expect(document.activeElement).toBe(input);
});
