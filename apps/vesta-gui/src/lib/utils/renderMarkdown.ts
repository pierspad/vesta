import { marked } from "marked";
import DOMPurify from "dompurify";

/** Render remote release notes using a restricted set of document elements. */
export function renderMarkdown(source: string): string {
  const parsed = marked.parse(source, { async: false, gfm: true });
  const fragment = DOMPurify.sanitize(parsed, {
    RETURN_DOM_FRAGMENT: true,
    ALLOWED_TAGS: ["p", "br", "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "li",
      "strong", "em", "del", "s", "a", "code", "pre", "blockquote", "hr",
      "table", "thead", "tbody", "tr", "th", "td"],
    ALLOWED_ATTR: ["href", "title", "start"],
    ALLOWED_URI_REGEXP: /^(?:https?:\/\/|mailto:|#)/i,
    ALLOW_DATA_ATTR: false,
    ALLOW_ARIA_ATTR: false,
  });
  for (const link of fragment.querySelectorAll("a[href]")) {
    link.setAttribute("target", "_blank");
    link.setAttribute("rel", "noopener noreferrer");
  }
  const container = document.createElement("div");
  container.append(fragment);
  return container.innerHTML;
}
