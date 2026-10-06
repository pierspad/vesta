import { describe, expect, it } from 'vitest';
import { compareVersions, officialReleaseUrl } from './updateVersion';
describe('update versions', () => {
  it('offers the stable release to users on its prerelease', () => {
    expect(compareVersions('v0.25.1', '0.25.1-dev.1')).toBeGreaterThan(0);
    expect(compareVersions('0.25.1-dev.2', '0.25.1-dev.10')).toBeLessThan(0);
    expect(compareVersions('0.25.1-dev.1', '0.25.1')).toBeLessThan(0);
  });
  it('handles major/minor/patch ordering and build metadata', () => {
    expect(compareVersions('1.0.0', '0.99.99')).toBeGreaterThan(0);
    expect(compareVersions('0.25.0', '0.26.0')).toBeLessThan(0);
    expect(compareVersions('v1.2.3+build1', '1.2.3+build2')).toBe(0);
  });
  it('rejects unavailable or malformed versions', () => {
    for (const value of ['', 'latest', '1.2', 'v1.2.3junk', '01.2.3']) expect(compareVersions(value, '1.2.3')).toBeNull();
  });
  it('accepts only official stable release pages', () => {
    expect(officialReleaseUrl('https://github.com/pierspad/vesta/releases/tag/v1.2.3')).toBeTruthy();
    for (const url of ['https://example.com/pierspad/vesta/releases/tag/v1.2.3', 'https://github.com/other/app/releases/tag/v1.2.3', 'https://github.com/pierspad/vesta/releases/tag/v1.2.3-dev.1', 'https://user@github.com/pierspad/vesta/releases/tag/v1.2.3', 'javascript:alert(1)']) expect(officialReleaseUrl(url)).toBeNull();
  });
});

describe('installer availability', () => {
  it('shows an install action only for a verified compatible released asset', async () => {
    const { selectInstaller } = await import('./updateVersion');
    const asset = { name: 'vesta_1.2.3_x64-setup.exe', browser_download_url: 'https://github.com/pierspad/vesta/releases/download/v1.2.3/vesta_1.2.3_x64-setup.exe', digest: `sha256:${'a'.repeat(64)}`, size: 100 };
    expect(selectInstaller([asset], 'windows', 'x86_64', 'v1.2.3')).toBe(asset);
    expect(selectInstaller([asset], 'windows', 'aarch64', 'v1.2.3')).toBeUndefined();
    expect(selectInstaller([asset], 'aur', 'x86_64', 'v1.2.3')).toBeUndefined();
    expect(selectInstaller([{ ...asset, digest: undefined }], 'windows', 'x86_64', 'v1.2.3')).toBeUndefined();
    expect(selectInstaller([{ ...asset, browser_download_url: 'https://example.com/installer.exe' }], 'windows', 'x86_64', 'v1.2.3')).toBeUndefined();
    expect(selectInstaller([], 'windows', 'x86_64', 'v1.2.3')).toBeUndefined();
  });
});
