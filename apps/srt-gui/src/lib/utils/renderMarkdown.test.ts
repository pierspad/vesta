// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { renderMarkdown } from "./renderMarkdown";

function render(source: string) {
  const root = document.createElement("div");
  root.innerHTML = renderMarkdown(source);
  return root;
}

describe("release Markdown", () => {
  it("formats release headings, lists, emphasis and commit links", () => {
    const root = render("## [0.28.0](https://github.com/pierspad/vesta/releases/tag/v0.28.0)\n\n### Fixes\n\n* **audio:** improved previews ([123](https://github.com/pierspad/vesta/commit/123))\n* Second fix");
    expect(root.querySelector("h2 a")?.textContent).toBe("0.28.0");
    expect(root.querySelector("h3")?.textContent).toBe("Fixes");
    expect(root.querySelectorAll("li")).toHaveLength(2);
    expect(root.querySelector("strong")?.textContent).toBe("audio:");
    expect(root.querySelector("li a")?.getAttribute("href")).toContain("/commit/123");
    for (const link of root.querySelectorAll("a")) {
      expect(link.target).toBe("_blank");
      expect(link.rel).toBe("noopener noreferrer");
    }
  });
  it("formats code, quotes and GitHub tables", () => {
    const root = render('> Note\n\nUse `vesta --help`.\n\n```sh\nvesta --help\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |');
    expect(root.querySelector("blockquote")?.textContent).toContain("Note");
    expect(root.querySelector("pre code")?.textContent).toContain("vesta --help");
    expect(root.querySelectorAll("td")).toHaveLength(2);
  });
  it("removes active HTML, event handlers and dangerous or local URL schemes", () => {
    const root = render('<script>alert(1)</script><iframe src="https://evil.test"></iframe><img src="https://evil.test/pixel" onerror="alert(1)"><svg onload="alert(1)"></svg>\n\n[bad](javascript:alert%281%29) [local](file:///etc/passwd) [data](data:text/html,test)\n\n<a href="https://example.com" onclick="alert(1)" style="position:fixed">safe</a>');
    expect(root.querySelector("script, iframe, img, svg, [onclick], [onerror], [style]")).toBeNull();
    expect([...root.querySelectorAll("a[href]")].map(link => link.getAttribute("href"))).toEqual(["https://example.com"]);
  });
});
