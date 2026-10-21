/** JSON comments are removed only outside strings; escapes stay byte-for-byte intact. */
export function parseJsonWithComments(source: string): unknown {
  return JSON.parse(source.replace(/"(?:\\.|[^"\\])*"|\/\/[^\r\n]*|\/\*[\s\S]*?\*\//g,
    token => token.startsWith('"') ? token : token.replace(/[^\r\n]/g, " ")));
}
