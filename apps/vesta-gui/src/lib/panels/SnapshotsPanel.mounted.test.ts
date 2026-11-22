// @vitest-environment jsdom
import { afterEach, expect, it } from "vitest";
import { mount, tick, unmount } from "svelte";
import SnapshotDimensionsHarness from "$lib/test/fixtures/SnapshotDimensionsHarness.svelte";
import { uiMode } from "$lib/stores/uiModeStore.svelte";
let instance: ReturnType<typeof mount> | undefined;
afterEach(async () => { if (instance) await unmount(instance); instance=undefined; document.body.replaceChildren(); uiMode.expertMode=false; });
it("links dimensions only while the lock is enabled and supports keyboard increments", async () => {
 uiMode.expertMode=true;
 const target=document.createElement("div");document.body.append(target);
 instance=mount(SnapshotDimensionsHarness,{target});
 await tick();
 const [quality,width,height]=[...target.querySelectorAll<HTMLInputElement>('input[type="number"]')];
 expect(quality.value).toBe("60");
 target.querySelector<HTMLButtonElement>('button[aria-pressed]')!.click(); await tick();
 width.dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowRight",shiftKey:true,bubbles:true,cancelable:true}));await tick();
 expect(width.value).toBe("650");expect(height.value).toBe("366");
 target.querySelector<HTMLButtonElement>('button[aria-pressed]')!.click();await tick();
 height.dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowDown",ctrlKey:true,bubbles:true,cancelable:true}));await tick();
 expect(height.value).toBe("266");expect(width.value).toBe("650");
});
it("keeps linked dimensions accurate through a burst of pointer presses", async () => {
 uiMode.expertMode=true;
 const target=document.createElement("div"); document.body.append(target);
 instance=mount(SnapshotDimensionsHarness,{target}); await tick();
 target.querySelector<HTMLButtonElement>('button[aria-pressed]')!.click(); await tick();
 const width=target.querySelectorAll<HTMLInputElement>('input[type="number"]')[1];
 const increase=width.parentElement!.querySelector("button")!;
 for(let i=0;i<30;i++) {
   increase.dispatchEvent(new MouseEvent("pointerdown",{button:0,bubbles:true,cancelable:true}));
   increase.dispatchEvent(new MouseEvent("click",{detail:i+1,bubbles:true}));
 }
 await tick();
 const height=target.querySelectorAll<HTMLInputElement>('input[type="number"]')[2];
 expect(width.value).toBe("670"); expect(height.value).toBe("377");
 expect(target.querySelector('button[aria-pressed="true"]')).not.toBeNull();
});
