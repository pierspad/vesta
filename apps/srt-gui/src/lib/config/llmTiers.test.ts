import { describe, it, expect } from 'vitest';
import { buildTiersPayload, countTiersAndEndpoints } from './llmTiers';
import type { ApiKeyConfig } from './apiKeys';
import type { Tier } from './translationTiers';

describe('llmTiers', () => {
  const mockApiKeys: ApiKeyConfig[] = [
    {
      id: 'key-google-1',
      apiType: 'google',
      name: 'My Gemini Key',
      apiKey: 'AIzaSyTestKey123',
      apiUrl: '',
    },
    {
      id: 'key-openai-1',
      apiType: 'openai',
      name: 'My OpenAI Key',
      apiKey: 'sk-test456',
      apiUrl: '',
    },
    {
      id: 'key-empty-1',
      apiType: 'anthropic',
      name: 'Empty Key',
      apiKey: '',
      apiUrl: '',
    },
  ];

  it('buildTiersPayload resolves valid API keys and providers', () => {
    const tiers: Tier[] = [
      {
        id: 'tier-1',
        entries: [
          {
            id: 'entry-1',
            apiKeyId: 'key-google-1',
            provider: 'google',
            model: 'gemini-2.5-flash',
            rpm: 15,
            maxRequests: 100,
          },
          {
            id: 'entry-2',
            apiKeyId: 'key-openai-1',
            provider: 'openai',
            model: 'gpt-4o-mini',
          },
        ],
      },
      {
        id: 'tier-2',
        entries: [
          {
            id: 'entry-3',
            apiKeyId: 'key-empty-1', // Missing key -> must be skipped
            provider: 'anthropic',
            model: 'claude-3-5-haiku',
          },
        ],
      },
    ];

    const payload = buildTiersPayload(tiers, mockApiKeys);
    expect(payload).not.toBeNull();
    expect(payload!.length).toBe(1); // Tier 2 has only invalid entries, so it gets dropped
    expect(payload![0].length).toBe(2);

    expect(payload![0][0]).toEqual({
      provider: 'google',
      model: 'gemini-2.5-flash',
      api_key: 'AIzaSyTestKey123',
      api_url: 'https://generativelanguage.googleapis.com/v1beta',
      rpm: 15,
      max_requests: 100,
    });
    expect(payload![0][1].model).toBe('gpt-4o-mini');
  });

  it('buildTiersPayload allows local providers without API keys', () => {
    const tiers: Tier[] = [
      {
        id: 'tier-local',
        entries: [
          {
            id: 'e-local',
            apiKeyId: 'non-existent-id',
            provider: 'local',
            model: 'qwen2.5:7b',
          },
        ],
      },
    ];

    const payload = buildTiersPayload(tiers, []);
    expect(payload).not.toBeNull();
    expect(payload![0][0].provider).toBe('local');
    expect(payload![0][0].model).toBe('qwen2.5:7b');
    expect(payload![0][0].api_key).toBeNull();
  });

  it('countTiersAndEndpoints counts properly configured entries', () => {
    const tiers: Tier[] = [
      {
        id: 't1',
        entries: [
          { id: '1', apiKeyId: '', provider: 'google', model: 'model-a' },
          { id: '2', apiKeyId: '', provider: 'google', model: 'model-b' },
        ],
      },
      {
        id: 't2',
        entries: [
          { id: '3', apiKeyId: '', provider: 'openai', model: 'model-c' },
          { id: '4', apiKeyId: '', provider: 'openai', model: '' }, // empty model ignored
        ],
      },
      {
        id: 't3',
        entries: [], // empty tier ignored
      },
    ];

    const count = countTiersAndEndpoints(tiers);
    expect(count.tiers).toBe(2);
    expect(count.endpoints).toBe(3);
  });
});
