import { beforeEach, describe, expect, it, vi } from 'vitest';
import { persistStorage } from './persistStorage';
import { strongholdSecretsStore } from './strongholdSecretsStore';

const NAME = 'dungeon-master-ai';

function stateValue(settings: Record<string, unknown>, onboardingCompleted: boolean) {
  return {
    state: {
      settings,
      session: {},
      onboarding: { completed: onboardingCompleted },
      pc: {},
    },
    version: 0,
  } as Parameters<typeof persistStorage.setItem>[1];
}

describe('persistStorage', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('round-trips plaintext settings through setItem then getItem', async () => {
    await persistStorage.setItem(
      NAME,
      stateValue({ systemPrompt: 'be a good DM', temperature: 0.7 }, true),
    );
    const loaded = await persistStorage.getItem(NAME);
    expect(loaded?.state.settings?.systemPrompt).toBe('be a good DM');
    expect(loaded?.state.onboarding?.completed).toBe(true);
  });

  it('flushes settings.json even when a Stronghold write rejects', async () => {
    vi.spyOn(strongholdSecretsStore, 'set').mockRejectedValue(new Error('vault locked'));
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    await persistStorage.setItem(
      NAME,
      stateValue(
        {
          systemPrompt: 'survives the vault failure',
          providers: { anthropic: null, 'openai-compat': null, 'local-mistralrs': null },
        },
        true,
      ),
    );

    const loaded = await persistStorage.getItem(NAME);
    expect(loaded?.state.settings?.systemPrompt).toBe('survives the vault failure');
    expect(loaded?.state.onboarding?.completed).toBe(true);
    expect(consoleError).toHaveBeenCalled();
  });

  it('loads plaintext settings even when the Stronghold vault fails to open', async () => {
    await persistStorage.setItem(
      NAME,
      stateValue({ systemPrompt: 'plaintext is independent' }, true),
    );
    vi.spyOn(strongholdSecretsStore, 'get').mockRejectedValue(new Error('vault corrupt'));
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    const loaded = await persistStorage.getItem(NAME);
    expect(loaded?.state.settings?.systemPrompt).toBe('plaintext is independent');
    expect(consoleError).toHaveBeenCalled();
  });
});
