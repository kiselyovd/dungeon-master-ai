import { beforeEach, describe, expect, it } from 'vitest';
import { strongholdSecretsStore } from '../strongholdSecretsStore';

describe('strongholdSecretsStore', () => {
  beforeEach(() => {
    strongholdSecretsStore._resetForTests();
  });

  it('returns undefined for missing keys', async () => {
    expect(await strongholdSecretsStore.get('absent')).toBeUndefined();
  });

  it('round-trips a JSON-serialisable value through the encrypted store', async () => {
    await strongholdSecretsStore.set('providers', {
      anthropic: { apiKey: 'sk-ant-test', model: 'claude-haiku' },
    });
    const back = await strongholdSecretsStore.get('providers');
    expect(back).toEqual({
      anthropic: { apiKey: 'sk-ant-test', model: 'claude-haiku' },
    });
  });

  it('delete removes the key so subsequent get returns undefined', async () => {
    await strongholdSecretsStore.set('replicate_api_key', 'r8_xyz');
    await strongholdSecretsStore.delete('replicate_api_key');
    expect(await strongholdSecretsStore.get('replicate_api_key')).toBeUndefined();
  });

  it('save resolves without error after writes', async () => {
    await strongholdSecretsStore.set('providers', { foo: 'bar' });
    await expect(strongholdSecretsStore.save()).resolves.toBeUndefined();
  });
});
