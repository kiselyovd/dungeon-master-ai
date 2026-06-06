import { describe, expect, it } from 'vitest';
import { CHAT_CATALOG } from '../api/providers-catalog';
import type { ProviderKind } from '../state/providers';

/**
 * Contract guard for M11 Batch D.5: native Anthropic was removed and cloud chat
 * was consolidated onto the generic OpenAI-compatible provider. This test fails
 * loudly if an `anthropic` provider entry ever creeps back into the chat catalog
 * (the durable, type-erased surface that a regression would slip through).
 */
describe('no native anthropic provider (M11 Batch D.5)', () => {
  it('CHAT_CATALOG has no anthropic entry', () => {
    expect(CHAT_CATALOG.every((e) => e.id !== 'anthropic')).toBe(true);
  });

  it('CHAT_CATALOG ids are limited to the supported provider kinds', () => {
    const supported: ProviderKind[] = ['openai-compat', 'local-mistralrs'];
    for (const entry of CHAT_CATALOG) {
      expect(supported).toContain(entry.id as ProviderKind);
    }
  });
});
