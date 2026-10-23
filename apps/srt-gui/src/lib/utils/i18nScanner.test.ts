import { describe, expect, it } from "vitest";
import { extractReferencedTranslationKeys, findHardcodedTemplateText, findHardcodedExpressionText } from "../../../scripts/i18n-scanner.mjs";

describe("i18n source scanner helpers", () => {
  it("keeps scanning translation keys in test and fixture sources", () => {
    expect([...extractReferencedTranslationKeys(
      'const label = t("about.sponsor"); setupT("setup.compressed");',
      "src/lib/test/fixtures/Example.svelte",
    )]).toEqual(["about.sponsor", "setup.compressed"]);
  });

  it("ignores fixture copy only for the hardcoded-template heuristic", () => {
    expect(findHardcodedTemplateText(
      "src/lib/test/fixtures/Example.svelte",
      "<button>Fixture-only example</button>",
    )).toEqual([]);
    expect(findHardcodedTemplateText(
      "src/lib/components/Example.test.svelte",
      "<button>Fixture-only example</button>",
    )).toEqual(["Fixture-only example"]);
  });

  it("allows only the reviewed exact technical literals", () => {
    const text = `<p>Opus · {t("setup.compressed")}</p>
      <button>AnkiConnect</button><p>Silero Voice Activity Detection 6.2.0</p>
      <span>-14 LUFS</span><dt>GStreamer</dt><b>Vesta</b>`;
    expect(findHardcodedTemplateText("src/Real.svelte", text)).toEqual([]);
  });

  it("still reports nearby explanatory copy and ordinary untranslated UI", () => {
    expect(findHardcodedTemplateText(
      "src/Real.svelte",
      "<p>GStreamer is optional</p><button>Transcribe now</button><span>-14 LUFS target</span>",
    )).toEqual(["GStreamer is optional", "Transcribe now", "-14 LUFS target"]);
  });
});

it("finds untranslated conditional copy and tooltips without flagging code attributes", () => {
 expect(findHardcodedExpressionText("src/Real.svelte", `<button class="flex items-center" title={open ? "Collapse sidebar" : "Expand sidebar"}>{playing ? t("common.pause") : "Play audio"}</button><input placeholder="Type a name" /><img alt="Preview frame" />`)).toEqual(["Collapse sidebar", "Expand sidebar", "Play audio", "Type a name", "Preview frame"]);
});
