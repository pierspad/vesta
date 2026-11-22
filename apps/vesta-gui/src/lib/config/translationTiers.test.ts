import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as vestaConfig from '$lib/config/vestaConfig';
import { loadTiers, type Tier } from './translationTiers';

describe('translationTiers', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('normalizes malformed shared tier fields and skips invalid tiers and entries', () => {
    vi.spyOn(vestaConfig, 'getItem').mockReturnValue(JSON.stringify([
      null,
      { id: 42, entries: 'invalid' },
      { id: 7, entries: [null, {}, { provider: 12 }, {
        provider: 'google', apiKeyId: 9, model: false, rpm: -1, maxRequests: '10',
      }] },
    ]));

    const tiers: Tier[] = loadTiers();
    expect(tiers).toHaveLength(1);
    expect(tiers[0].id).toMatch(/^tier-/);
    expect(tiers[0].entries).toHaveLength(1);
    expect(tiers[0].entries[0]).toMatchObject({ provider: 'google', apiKeyId: '', model: '' });
    expect(tiers[0].entries[0].id).toMatch(/^te-/);
    expect(tiers[0].entries[0].rpm).toBeUndefined();
    expect(tiers[0].entries[0].maxRequests).toBeUndefined();
  });
});
