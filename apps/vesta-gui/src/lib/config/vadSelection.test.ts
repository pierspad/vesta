import { describe, it, expect, beforeEach, vi } from 'vitest';
import { loadVadSelection, saveVadSelection, DEFAULT_VAD_MODEL_ID } from './vadSelection';
import * as vestaConfig from '$lib/config/vestaConfig';

describe('vadSelection', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('returns default when no selection is stored', () => {
    vi.spyOn(vestaConfig, 'getItem').mockReturnValue(null);
    const sel = loadVadSelection();
    expect(sel.modelId).toBe(DEFAULT_VAD_MODEL_ID);
    expect(sel.customPath).toBeNull();
  });

  it('loads valid stored built-in selection', () => {
    vi.spyOn(vestaConfig, 'getItem').mockReturnValue(JSON.stringify({ modelId: 'v4.0.0', customPath: null }));
    const sel = loadVadSelection();
    expect(sel.modelId).toBe('v4.0.0');
    expect(sel.customPath).toBeNull();
  });

  it('loads valid stored custom path selection', () => {
    vi.spyOn(vestaConfig, 'getItem').mockReturnValue(JSON.stringify({ modelId: 'v5.1.2', customPath: '/path/to/custom_vad.bin' }));
    const sel = loadVadSelection();
    expect(sel.modelId).toBe('v5.1.2');
    expect(sel.customPath).toBe('/path/to/custom_vad.bin');
  });

  it('handles corrupted JSON gracefully', () => {
    vi.spyOn(vestaConfig, 'getItem').mockReturnValue('invalid-json{{{');
    const sel = loadVadSelection();
    expect(sel.modelId).toBe(DEFAULT_VAD_MODEL_ID);
    expect(sel.customPath).toBeNull();
  });

  it('saves selection to vestaConfig', () => {
    const spy = vi.spyOn(vestaConfig, 'setItem').mockImplementation(() => {});
    saveVadSelection({ modelId: 'v5.1.2', customPath: '/my/vad.bin' });
    expect(spy).toHaveBeenCalledWith(
      'vesta-transcribe-vad-selection',
      JSON.stringify({ modelId: 'v5.1.2', customPath: '/my/vad.bin' })
    );
  });
});
