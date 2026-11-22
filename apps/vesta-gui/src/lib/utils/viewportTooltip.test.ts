// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { viewportTooltip } from "./viewportTooltip";
let destroy = () => {};
afterEach(() => { destroy(); document.body.replaceChildren(); vi.restoreAllMocks(); });
it("portals out of clipping and clamps to the right and bottom window edges", () => {
 const anchor=document.createElement("div"), bubble=document.createElement("div"); anchor.append(bubble); document.body.append(anchor);
 vi.spyOn(anchor,"getBoundingClientRect").mockReturnValue({left:window.innerWidth-40,top:window.innerHeight-30,width:40,bottom:window.innerHeight-10} as DOMRect);
 vi.spyOn(bubble,"getBoundingClientRect").mockReturnValue({width:280,height:50} as DOMRect);
 const action=viewportTooltip(bubble); destroy=action.destroy;
 expect(bubble.parentElement).toBe(document.body);
 anchor.dispatchEvent(new Event("mouseenter"));
 expect(bubble.style.opacity).toBe("1");
 expect(Number.parseFloat(bubble.style.left)+280).toBeLessThanOrEqual(window.innerWidth-8);
 expect(Number.parseFloat(bubble.style.top)+50).toBeLessThanOrEqual(window.innerHeight-8);
 anchor.dispatchEvent(new Event("mouseleave")); expect(bubble.style.display).toBe("none");
 action.update({visible:true}); expect(bubble.style.display).toBe("block");
 window.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape"})); expect(bubble.style.display).toBe("none");
});
it("flips below its trigger when there is insufficient space above", () => {
 const anchor=document.createElement("div"), bubble=document.createElement("div"); anchor.append(bubble); document.body.append(anchor);
 vi.spyOn(anchor,"getBoundingClientRect").mockReturnValue({left:0,top:0,width:30,bottom:30} as DOMRect);
 vi.spyOn(bubble,"getBoundingClientRect").mockReturnValue({width:200,height:50} as DOMRect);
 const action=viewportTooltip(bubble,{visible:true}); destroy=action.destroy;
 expect(bubble.style.left).toBe("8px"); expect(bubble.style.top).toBe("38px");
});
