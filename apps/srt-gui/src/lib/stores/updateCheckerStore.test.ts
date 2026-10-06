import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
const mocks = vi.hoisted(() => ({ fetch: vi.fn(), invoke: vi.fn(), listen: vi.fn(), open: vi.fn(), snackbar: vi.fn() }));
vi.mock('$lib/services/tauriClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('$lib/services/tauriHttp', () => ({ fetch: mocks.fetch }));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));
vi.mock('@tauri-apps/plugin-shell', () => ({ open: mocks.open }));
vi.mock('$lib/stores/snackbarStore.svelte', () => ({ snackbar: { show: mocks.snackbar } }));
vi.mock('$lib/i18n', () => ({ t: (key: string) => key }));
vi.mock('$lib/config/vestaConfig', () => ({ getItem: () => 'false', setItem: vi.fn() }));
const release = (assets: unknown[] = []) => ({ tag_name: 'v1.2.3', html_url: 'https://github.com/pierspad/vesta/releases/tag/v1.2.3', assets });
const response = (data: unknown) => ({ ok: true, json: async () => data });
async function store() {
  const { updateCheckerStore } = await import('./updateCheckerStore.svelte');
  updateCheckerStore.appVersionNum = 'v1.2.3-dev.1';
  return updateCheckerStore;
}
beforeEach(() => {
  vi.resetModules();
  Object.values(mocks).forEach(mock => mock.mockReset());
  vi.stubGlobal('navigator', { onLine: true });
  vi.spyOn(console, 'warn').mockImplementation(() => {});
  vi.spyOn(console, 'error').mockImplementation(() => {});
});
afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks(); });
describe('update checker', () => {
  it('offers the stable version to a prerelease without inventing an installer', async () => {
    mocks.fetch.mockResolvedValue(response(release()));
    const updates = await store();
    updates.installation = { channel: 'windows', os: 'windows', arch: 'x86_64' };
    await updates.checkForUpdates();
    expect(updates.updateStatus).toBe('available');
    expect(updates.canInstall).toBe(false);
  });
  it('reports failure when the installed version is unknown', async () => {
    mocks.fetch.mockResolvedValueOnce(response(release())).mockRejectedValueOnce(new Error('offline'));
    const updates = await store();
    updates.appVersionNum = '';
    await updates.checkForUpdates();
    expect(updates.updateStatus).toBe('error');
  });
  it('uses the official release redirect, never an unpublished package version', async () => {
    mocks.fetch.mockRejectedValueOnce(new Error('rate limited')).mockResolvedValueOnce({ status: 302, ok: false, headers: { get: () => 'https://github.com/pierspad/vesta/releases/tag/v1.2.3' } });
    const updates = await store();
    await updates.checkForUpdates();
    expect(updates.updateStatus).toBe('available');
    expect(mocks.fetch.mock.calls.map(call => call[0])).toEqual(['https://api.github.com/repos/pierspad/vesta/releases/latest', 'https://github.com/pierspad/vesta/releases/latest']);
  });
  it('deduplicates concurrent update checks', async () => {
    let resolve!: (value: unknown) => void;
    mocks.fetch.mockReturnValue(new Promise(r => { resolve = r; }));
    const updates = await store();
    const first = updates.checkForUpdates();
    await updates.checkForUpdates();
    expect(mocks.fetch).toHaveBeenCalledTimes(1);
    resolve(response(release()));
    await first;
    expect(updates.updateStatus).toBe('available');
  });
  it('allows only direct installers and cleans up listeners after failure', async () => {
    const asset = { name: 'vesta_1.2.3_x64-setup.exe', size: 100, digest: `sha256:${'a'.repeat(64)}`, browser_download_url: 'https://github.com/pierspad/vesta/releases/download/v1.2.3/vesta_1.2.3_x64-setup.exe' };
    mocks.fetch.mockResolvedValue(response(release([asset])));
    const updates = await store();
    updates.installation = { channel: 'aur', os: 'linux', arch: 'x86_64' };
    await updates.checkForUpdates();
    expect(updates.canInstall).toBe(false);
    expect(updates.managerHint).toBe('settings.updatesViaAur');
    updates.installation = { channel: 'windows', os: 'windows', arch: 'x86_64' };
    expect(updates.canInstall).toBe(true);
    const unlisten = vi.fn();
    mocks.listen.mockResolvedValue(unlisten);
    mocks.invoke.mockRejectedValue(new Error('checksum mismatch'));
    await updates.installUpdate();
    expect(mocks.invoke).toHaveBeenCalledWith('install_release_update', { version: 'v1.2.3' });
    expect(updates.installing).toBe(false);
    expect(unlisten).toHaveBeenCalledOnce();
    expect(mocks.snackbar).toHaveBeenLastCalledWith('Error: checksum mismatch', 'error');
  });
  it('reports offline without making a network call', async () => {
    vi.stubGlobal('navigator', { onLine: false });
    const updates = await store();
    await updates.checkForUpdates();
    expect(updates.updateStatus).toBe('offline');
    expect(mocks.fetch).not.toHaveBeenCalled();
  });
});
