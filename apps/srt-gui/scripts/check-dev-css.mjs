// Exercise style requests BEFORE the component JS, including server restart.
import { createServer } from "vite";
import assert from "node:assert/strict";
const components = ["components/SearchableSelect", "components/CodeEditor", "tabs/FlashcardsTab"];
for (let restart = 0; restart < 2; restart++) {
  const server = await createServer({ server: { port: 0, watch: null }, logLevel: "warn" });
  await server.listen();
  try {
    for (const component of components) {
      const url = `/src/lib/${component}.svelte?svelte&type=style&lang.css`;
      const result = await server.transformRequest(url);
      assert.ok(result?.code, `Missing CSS for ${component}`);
      assert.ok(!result.code.includes('import type {'), `Svelte source leaked as CSS: ${component}`);
      assert.ok(result.code.includes("svelte-"), `Missing scoped styles for ${component}`);
      const reloaded = await server.transformRequest(url);
      assert.equal(reloaded.code, result.code);
    }
  } finally {
    await server.close();
  }
}
console.log("Cold CSS requests, repeated requests and dev-server restart: passed.");
