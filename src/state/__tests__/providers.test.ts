import * as v from 'valibot';
import { describe, expect, it } from 'vitest';
import {
  AnthropicConfigSchema,
  OpenaiCompatConfigSchema,
  ProviderConfigSchema,
  parseApiKey,
  parseBaseUrl,
} from '../providers';

describe('parseApiKey', () => {
  it('accepts and trims non-empty strings', () => {
    expect(parseApiKey('  sk-test  ')).toBe('sk-test');
  });

  it('rejects empty / whitespace-only', () => {
    expect(parseApiKey('')).toBeNull();
    expect(parseApiKey('   ')).toBeNull();
  });
});

describe('parseBaseUrl', () => {
  it('accepts http and https URLs', () => {
    expect(parseBaseUrl('http://localhost:1234')).toBe('http://localhost:1234');
    expect(parseBaseUrl('https://api.openrouter.ai')).toBe('https://api.openrouter.ai');
  });

  it('trims whitespace before validating', () => {
    expect(parseBaseUrl('  https://x.test  ')).toBe('https://x.test');
  });

  it('rejects non-URL input', () => {
    expect(parseBaseUrl('')).toBeNull();
    expect(parseBaseUrl('not a url')).toBeNull();
  });

  it('rejects file:// and other non-http schemes', () => {
    expect(parseBaseUrl('file:///etc/passwd')).toBeNull();
    expect(parseBaseUrl('ftp://example.com')).toBeNull();
    expect(parseBaseUrl('javascript:alert(1)')).toBeNull();
  });
});

describe('AnthropicConfigSchema', () => {
  it('accepts a valid config', () => {
    const ok = v.safeParse(AnthropicConfigSchema, {
      kind: 'anthropic',
      apiKey: 'sk-ant-test',
      model: 'claude-haiku-4-5-20251001',
    });
    expect(ok.success).toBe(true);
  });

  it('rejects empty api key', () => {
    const bad = v.safeParse(AnthropicConfigSchema, {
      kind: 'anthropic',
      apiKey: '   ',
      model: 'claude',
    });
    expect(bad.success).toBe(false);
  });
});

describe('OpenaiCompatConfigSchema', () => {
  it('accepts a localhost URL', () => {
    const ok = v.safeParse(OpenaiCompatConfigSchema, {
      kind: 'openai-compat',
      baseUrl: 'http://localhost:1234',
      apiKey: 'sk-x',
      model: 'qwen3-1.7b',
    });
    expect(ok.success).toBe(true);
  });

  it('rejects non-URL base', () => {
    const bad = v.safeParse(OpenaiCompatConfigSchema, {
      kind: 'openai-compat',
      baseUrl: 'banana',
      apiKey: 'sk-x',
      model: 'qwen3',
    });
    expect(bad.success).toBe(false);
  });
});

describe('ProviderConfigSchema discriminated union', () => {
  it('routes by kind', () => {
    const anthropic = v.safeParse(ProviderConfigSchema, {
      kind: 'anthropic',
      apiKey: 'sk-ant',
      model: 'claude-haiku',
    });
    expect(anthropic.success).toBe(true);

    const openai = v.safeParse(ProviderConfigSchema, {
      kind: 'openai-compat',
      baseUrl: 'http://localhost:1234',
      apiKey: 'sk-x',
      model: 'qwen3',
    });
    expect(openai.success).toBe(true);
  });

  it('rejects unknown kind', () => {
    const bad = v.safeParse(ProviderConfigSchema, {
      kind: 'telepathic',
      apiKey: 'sk-x',
    });
    expect(bad.success).toBe(false);
  });
});
