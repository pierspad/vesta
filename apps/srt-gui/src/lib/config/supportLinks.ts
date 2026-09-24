export const supportLinks = [
  { id: "github", label: "GitHub Sponsors", url: "https://github.com/sponsors/pierspad" },
  { id: "coffee", label: "Buy Me a Coffee", url: "https://buymeacoffee.com/pierspad" },
  { id: "kofi", label: "Ko-fi", url: "https://ko-fi.com/pierspad" },
] as const;

export function isTrustedSupportUrl(url: string): boolean {
  try {
    const { protocol, hostname } = new URL(url);
    return protocol === "https:" && ["github.com", "buymeacoffee.com", "ko-fi.com"].includes(hostname);
  } catch {
    return false;
  }
}
