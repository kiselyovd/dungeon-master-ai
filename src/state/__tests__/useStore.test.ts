import { beforeEach, describe, expect, it } from 'vitest';
import type { ApiKey, BaseUrl, OpenaiCompatConfig } from '../providers';
import { useStore } from '../useStore';

describe('useStore - chat slice', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  it('starts with empty messages and no streaming state', () => {
    const s = useStore.getState().chat;
    expect(s.messages).toEqual([]);
    expect(s.streamingAssistant).toBeNull();
    expect(s.isStreaming).toBe(false);
    expect(s.lastError).toBeNull();
  });

  it('appends user and assistant messages, finalising the streaming buffer', () => {
    useStore.getState().chat.appendUser('hello');
    useStore.getState().chat.appendAssistantDelta('hi ');
    useStore.getState().chat.appendAssistantDelta('there');
    useStore.getState().chat.finalizeAssistant();

    const messages = useStore.getState().chat.messages;
    expect(messages).toHaveLength(2);
    expect(messages[0]).toMatchObject({ role: 'user', content: 'hello' });
    expect(messages[1]).toMatchObject({ role: 'assistant', content: 'hi there' });
    // Each message gets a stable id (uuid or fallback).
    expect(typeof messages[0]?.id).toBe('string');
    expect(messages[0]?.id).not.toBe(messages[1]?.id);
  });

  it('finalizeAssistant is a no-op when no stream is in progress', () => {
    useStore.getState().chat.finalizeAssistant();
    expect(useStore.getState().chat.messages).toEqual([]);
  });

  it('beginStream/endStream toggles isStreaming and tracks the controller', () => {
    const controller = new AbortController();
    useStore.getState().chat.beginStream(controller);
    expect(useStore.getState().chat.isStreaming).toBe(true);
    expect(useStore.getState().chat.abortController).toBe(controller);

    useStore.getState().chat.endStream();
    expect(useStore.getState().chat.isStreaming).toBe(false);
    expect(useStore.getState().chat.abortController).toBeNull();
  });

  it('abort triggers the active controller and is idempotent', () => {
    const controller = new AbortController();
    useStore.getState().chat.beginStream(controller);
    useStore.getState().chat.abort();
    expect(controller.signal.aborted).toBe(true);

    // Second call is harmless.
    useStore.getState().chat.abort();
    expect(controller.signal.aborted).toBe(true);
  });
});

describe('useStore - settings slice', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  it('starts with openai-compat active and no provider configs', () => {
    const s = useStore.getState().settings;
    expect(s.activeProvider).toBe('openai-compat');
    expect(s.providers['openai-compat']).toBeNull();
    expect(s.providers['local-mistralrs']).toBeNull();
    expect(s.uiLanguage).toBe('en');
    expect(s.narrationLanguage).toBe('en');
  });

  it('setProviderConfig stores per-kind config without affecting other kinds', () => {
    const cfg: OpenaiCompatConfig = {
      kind: 'openai-compat',
      baseUrl: 'https://openrouter.ai/api/v1' as BaseUrl,
      apiKey: 'sk-test' as ApiKey,
      model: 'anthropic/claude-3.5-sonnet',
    };
    useStore.getState().settings.setProviderConfig(cfg);
    expect(useStore.getState().settings.providers['openai-compat']).toEqual(cfg);
    expect(useStore.getState().settings.providers['local-mistralrs']).toBeNull();
  });

  it('setUiLanguage updates only the language field', () => {
    useStore.getState().settings.setUiLanguage('ru');
    expect(useStore.getState().settings.uiLanguage).toBe('ru');
    expect(useStore.getState().settings.activeProvider).toBe('openai-compat');
  });

  it('clearProviderConfig nulls out the targeted kind only', () => {
    const cfg: OpenaiCompatConfig = {
      kind: 'openai-compat',
      baseUrl: 'https://openrouter.ai/api/v1' as BaseUrl,
      apiKey: 'sk-test' as ApiKey,
      model: 'anthropic/claude-3.5-sonnet',
    };
    useStore.getState().settings.setProviderConfig(cfg);
    useStore.getState().settings.clearProviderConfig('openai-compat');
    expect(useStore.getState().settings.providers['openai-compat']).toBeNull();
    expect(useStore.getState().settings.providers['local-mistralrs']).toBeNull();
  });
});
