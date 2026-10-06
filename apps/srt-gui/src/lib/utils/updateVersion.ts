// Stable releases and prereleases are compared without discarding prerelease identity.
const VERSION = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z.-]+)?$/i;
export function compareVersions(left: string, right: string): number | null {
  const a = VERSION.exec(left.trim());
  const b = VERSION.exec(right.trim());
  if (!a || !b) return null;
  for (let i = 1; i <= 3; i++) {
    const diff = Number(a[i]) - Number(b[i]);
    if (diff) return diff;
  }
  if (a[4] === b[4]) return 0;
  if (!a[4]) return 1;
  if (!b[4]) return -1;
  const ap = a[4].split('.');
  const bp = b[4].split('.');
  for (let i = 0; i < Math.max(ap.length, bp.length); i++) {
    if (ap[i] === undefined) return -1;
    if (bp[i] === undefined) return 1;
    if (ap[i] === bp[i]) continue;
    const an = /^\d+$/.test(ap[i]);
    const bn = /^\d+$/.test(bp[i]);
    if (an && bn) return Number(ap[i]) - Number(bp[i]);
    if (an !== bn) return an ? -1 : 1;
    return ap[i] < bp[i] ? -1 : 1;
  }
  return 0;
}

export function officialReleaseUrl(url: string | undefined): string | null {
  if (!url) return null;
  try {
    const parsed = new URL(url);
    return parsed.origin === 'https://github.com' && !parsed.username && !parsed.password &&
      /^\/pierspad\/vesta\/releases\/tag\/v?\d+\.\d+\.\d+$/.test(parsed.pathname)
      ? parsed.href : null;
  } catch { return null; }
}

export interface UpdateAsset {
  name: string;
  browser_download_url: string;
  digest?: string | null;
  size: number;
}
export function selectInstaller(assets: UpdateAsset[], channel: string, arch: string, version: string): UpdateAsset | undefined {
  const extension = { windows: '.exe', deb: '.deb', rpm: '.rpm' }[channel];
  const markers = arch === 'x86_64' ? ['x64', 'x86_64', 'amd64'] : arch === 'aarch64' ? ['arm64', 'aarch64'] : [];
  if (!extension || !markers.length || !/^v\d+\.\d+\.\d+$/.test(version)) return undefined;
  return assets.find(asset => {
    const name = asset.name.toLowerCase();
    return name.endsWith(extension) && markers.some(marker => name.includes(marker)) &&
      /^sha256:[a-f\d]{64}$/i.test(asset.digest || '') && asset.size > 0 && asset.size <= 512 * 1024 * 1024 &&
      asset.browser_download_url.startsWith(`https://github.com/pierspad/vesta/releases/download/${version}/`);
  });
}
