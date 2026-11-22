import { beforeEach, describe, it, expect, vi } from 'vitest';
import { buildTiersPayload, checkTiersAvailability, countTiersAndEndpoints } from './llmTiers';
import type { ApiKeyConfig } from './apiKeys';
import type { Tier } from './translationTiers';

vi.mock('$lib/services/modelDiscovery', () => ({
  fetchModelsFromEndpoint: vi.fn(),
}));
import { fetchModelsFromEndpoint } from '$lib/services/modelDiscovery';

describe('llmTiers', () => {
  beforeEach(() => vi.clearAllMocks());
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

  it('probes a shared local endpoint once per availability check', async () => {
    vi.mocked(fetchModelsFromEndpoint).mockResolvedValue([{ id: 'model-a', name: 'Model A' }]);
    const tiers: Tier[] = [{ id: 'tier', entries: [
      { id: 'a', apiKeyId: 'local-key', provider: 'local', model: 'model-a' },
      { id: 'b', apiKeyId: 'local-key', provider: 'local', model: 'model-b' },
    ] }];
    const keys: ApiKeyConfig[] = [{ id: 'local-key', apiType: 'local', name: 'Local', apiKey: '', apiUrl: 'http://localhost:11434' }];

    await expect(checkTiersAvailability(tiers, keys)).resolves.toEqual({ available: true });
    expect(fetchModelsFromEndpoint).toHaveBeenCalledTimes(1);
    expect(fetchModelsFromEndpoint).toHaveBeenCalledWith('http://localhost:11434', '', 2000);
  });

  it('keeps probes separate when credentials differ on the same endpoint', async () => {
    vi.mocked(fetchModelsFromEndpoint).mockResolvedValue([{ id: 'model-a', name: 'Model A' }]);
    const tiers: Tier[] = [{ id: 'tier', entries: [
      { id: 'a', apiKeyId: 'key-a', provider: 'custom', model: 'model-a' },
      { id: 'b', apiKeyId: 'key-b', provider: 'custom', model: 'model-b' },
    ] }];
    const keys: ApiKeyConfig[] = [
      { id: 'key-a', apiType: 'custom', name: 'A', apiKey: 'secret-a', apiUrl: 'http://localhost:8080' },
      { id: 'key-b', apiType: 'custom', name: 'B', apiKey: 'secret-b', apiUrl: 'http://localhost:8080' },
    ];

    await expect(checkTiersAvailability(tiers, keys)).resolves.toEqual({ available: true });
    expect(fetchModelsFromEndpoint).toHaveBeenCalledTimes(2);
  });
});
