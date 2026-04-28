import { describe, it, expect, beforeEach } from 'vitest';
import { useStore } from '../useStore';

describe('useStore', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  it('starts with empty chat messages', () => {
    expect(useStore.getState().chat.messages).toEqual([]);
  });

  it('appends user and assistant messages', () => {
    useStore.getState().chat.appendUser('hello');
    useStore.getState().chat.appendAssistantDelta('hi ');
    useStore.getState().chat.appendAssistantDelta('there');
    useStore.getState().chat.finalizeAssistant();

    const messages = useStore.getState().chat.messages;
    expect(messages).toHaveLength(2);
    expect(messages[0]).toEqual({ role: 'user', content: 'hello' });
    expect(messages[1]).toEqual({ role: 'assistant', content: 'hi there' });
  });

  it('settings store starts with no api key and en language', () => {
    const s = useStore.getState().settings;
    expect(s.anthropicApiKey).toBeUndefined();
    expect(s.uiLanguage).toBe('en');
    expect(s.narrationLanguage).toBe('en');
  });

  it('settings updates persist in store', () => {
    useStore.getState().settings.setApiKey('sk-test');
    useStore.getState().settings.setUiLanguage('ru');
    expect(useStore.getState().settings.anthropicApiKey).toBe('sk-test');
    expect(useStore.getState().settings.uiLanguage).toBe('ru');
  });
});
