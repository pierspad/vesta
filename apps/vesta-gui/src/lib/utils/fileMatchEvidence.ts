import type { SmartMatchingRules } from "$lib/stores/smartMatchingStore.svelte";

export interface EpisodeRuleEvidence {
  episode: number;
  pattern: string;
  ruleIndex: number;
  matched: string;
}

export interface FileMatchEvidence {
  path: string;
  baseKey: string;
  episodeRule: EpisodeRuleEvidence | null;
  language: string | null;
  hints: string[];
  removedTokens: string[];
}

export interface EpisodeMatchEvidence {
  method: "baseKey" | "episode" | "unmatched";
  files: FileMatchEvidence[];
}

export function findEpisodeRule(filename: string, patterns: (string | RegExp)[]): EpisodeRuleEvidence | null {
  const stem = filename.replace(/\.[^/.]+$/, "").normalize("NFKC");
  for (const [ruleIndex, pattern] of patterns.entries()) {
    try {
      // exec on a fresh regex avoids global/sticky lastIndex and String.match capture loss.
      const regex = pattern instanceof RegExp
        ? new RegExp(pattern.source, pattern.flags.replace(/[gy]/g, ""))
        : new RegExp(pattern, "i");
      const match = regex.exec(stem);
      const raw = match?.groups?.episode ?? match?.[1] ?? match?.[0]?.match(/\d{1,4}/)?.[0];
      if (!match || !raw || !/^\d{1,4}$/.test(raw)) continue;
      const episode = Number(raw);
      // Bare years/resolutions are common release metadata, not episode evidence.
      if (!/[e話话集期화#]|episode|folge|cap[ií]tulo|серия/i.test(match[0])
        && ((episode >= 1900 && episode <= 2099) || [360, 480, 576, 720, 1080, 2160, 4320].includes(episode))) continue;
      return { episode, pattern: regex.source, ruleIndex, matched: match[0] };
    } catch { /* Ignore invalid legacy/custom patterns. */ }
  }
  return null;
}

export function extractSeason(filename: string): number | null {
  const match = filename.normalize("NFKC").match(/(?:^|[\s._\-\[(])s(?:eason)?[\s._-]*(\d{1,2})(?=[\s._-]*e|[\s._\-\])]|$)/i)
    ?? filename.match(/(?:^|[\s._\-\[(])(\d{1,2})x\d/i);
  return match ? Number(match[1]) : null;
}

const escape = (value: string) => value.replace(/[.*+?^$()|[\]{}\\]/g, "\\$&");
export function matchingTokens(filename: string, tokens: string[]): string[] {
  return tokens.filter(token => token && new RegExp(
    `(?:^|[^\\p{L}\\p{N}])${token.split(/[\s._-]+/).map(escape).join("[\\s._-]+")}(?=$|[^\\p{L}\\p{N}])`, "iu",
  ).test(filename));
}

export function tokenEvidence(filename: string, rules: SmartMatchingRules) {
  return {
    hints: matchingTokens(filename, [...rules.originalSubtitleHints, ...rules.referenceSubtitleHints]),
    removedTokens: matchingTokens(filename, rules.removableNameTokens),
  };
}
