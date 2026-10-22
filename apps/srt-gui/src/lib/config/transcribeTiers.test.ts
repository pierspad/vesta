import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  loadTranscribeTiers,
  saveTranscribeTiers,
  transcribeTiersHaveUsableEntries,
  type TranscribeTier,
} from './transcribeTiers';
import * as vestaConfig from '$lib/config/vestaConfig';

describe('transcribeTiers', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('loadTranscribeTiers returns default when nothing is stored', () => {
    vi.spyOn(vestaConfig, 'getItem').mockReturnValue(null);
    const tiers = loadTranscribeTiers();
    expect(tiers.length).toBe(1);
    expect(tiers[0].entries.length).toBe(1);
    expect(tiers[0].entries[0].provider).toBe('local');
    expect(tiers[0].entries[0].model).toBe('base');
  });

  it('loadTranscribeTiers parses valid stored tiers', () => {
    const mockStored: TranscribeTier[] = [
      {
        id: 'tier-1',
        entries: [
          {
            id: 'e-1',
            provider: 'groq',
            apiKeyId: 'groq-key-1',
            model: 'whisper-large-v3',
            rpm: 20,
            maxRequests: 100,
          },
        ],
      },
    ];

    vi.spyOn(vestaConfig, 'getItem').mockReturnValue(JSON.stringify(mockStored));
    const tiers = loadTranscribeTiers();
    expect(tiers.length).toBe(1);
    expect(tiers[0].id).toBe('tier-1');
    expect(tiers[0].entries[0].model).toBe('whisper-large-v3');
    expect(tiers[0].entries[0].rpm).toBe(20);
  });

  it('loadTranscribeTiers handles corrupted JSON with default fallback', () => {
    vi.spyOn(vestaConfig, 'getItem').mockReturnValue('{ bad: json');
    const tiers = loadTranscribeTiers();
    expect(tiers.length).toBe(1);
    expect(tiers[0].entries[0].provider).toBe('local');
  });

  it('normalizes malformed shared tier fields and skips invalid tiers and entries', () => {
    vi.spyOn(vestaConfig, 'getItem').mockReturnValue(JSON.stringify([
      null,
      { id: 42, entries: 'invalid' },
      { id: 7, entries: [null, {}, { provider: 12 }, {
        provider: 'groq', apiKeyId: 9, model: false, rpm: -1, maxRequests: '10',
      }] },
    ]));
    const [tier] = loadTranscribeTiers();
    expect(tier.id).toMatch(/^tier-/);
    expect(tier.entries).toHaveLength(1);
    expect(tier.entries[0]).toMatchObject({ provider: 'groq', apiKeyId: '', model: '' });
    expect(tier.entries[0].id).toMatch(/^te-/);
    expect(tier.entries[0].rpm).toBeUndefined();
    expect(tier.entries[0].maxRequests).toBeUndefined();
  });

  it('transcribeTiersHaveUsableEntries checks for configured models', () => {
    const emptyTiers: TranscribeTier[] = [
      {
        id: 't1',
        entries: [
          { id: 'e1', provider: 'local', apiKeyId: '', model: '' },
          { id: 'e2', provider: 'groq', apiKeyId: '', model: '   ' },
        ],
      },
    ];
    expect(transcribeTiersHaveUsableEntries(emptyTiers)).toBe(false);

    const validTiers: TranscribeTier[] = [
      {
        id: 't2',
        entries: [{ id: 'e3', provider: 'local', apiKeyId: '', model: 'small' }],
      },
    ];
    expect(transcribeTiersHaveUsableEntries(validTiers)).toBe(true);
  });

  it('saveTranscribeTiers writes to vestaConfig', () => {
    const spy = vi.spyOn(vestaConfig, 'setItem').mockImplementation(() => {});
    const tiers: TranscribeTier[] = [
      {
        id: 't1',
        entries: [{ id: 'e1', provider: 'local', apiKeyId: '', model: 'medium' }],
      },
    ];
    saveTranscribeTiers(tiers);
    expect(spy).toHaveBeenCalledWith('vesta-transcribe-tiers', JSON.stringify(tiers));
  });
});
