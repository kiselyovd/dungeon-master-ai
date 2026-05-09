import { describe, expect, it } from 'vitest';
import { create } from 'zustand';
import {
  createSettingsSlice,
  DEFAULT_CHAT_WIDTH,
  MAX_CHAT_WIDTH,
  MIN_CHAT_WIDTH,
  type SettingsSlice,
} from '../settings';

function freshSettingsStore() {
  return create<SettingsSlice>()((...a) => ({
    ...createSettingsSlice(...a),
  }));
}

describe('SettingsSlice model tab fields', () => {
  it('has default systemPrompt as empty string', () => {
    const store = freshSettingsStore();
    expect(store.getState().settings.systemPrompt).toBe('');
  });

  it('has default temperature of 0.7', () => {
    const store = freshSettingsStore();
    expect(store.getState().settings.temperature).toBe(0.7);
  });

  it('has null replicateApiKey by default', () => {
    const store = freshSettingsStore();
    expect(store.getState().settings.replicateApiKey).toBeNull();
  });

  it('setSystemPrompt updates value', () => {
    const store = freshSettingsStore();
    store.getState().settings.setSystemPrompt('You are a DM.');
    expect(store.getState().settings.systemPrompt).toBe('You are a DM.');
  });

  it('setTemperature updates value', () => {
    const store = freshSettingsStore();
    store.getState().settings.setTemperature(1.2);
    expect(store.getState().settings.temperature).toBe(1.2);
  });

  it('setReplicateApiKey updates value', () => {
    const store = freshSettingsStore();
    store.getState().settings.setReplicateApiKey('r8_xyz');
    expect(store.getState().settings.replicateApiKey).toBe('r8_xyz');
  });
});

describe('SettingsSlice chat panel width', () => {
  it('exports MIN/MAX bounds at 360 and 640', () => {
    expect(MIN_CHAT_WIDTH).toBe(360);
    expect(MAX_CHAT_WIDTH).toBe(640);
  });

  it('defaults chatPanelWidth to 480', () => {
    const store = freshSettingsStore();
    expect(store.getState().settings.chatPanelWidth).toBe(DEFAULT_CHAT_WIDTH);
    expect(DEFAULT_CHAT_WIDTH).toBe(480);
  });

  it('setChatPanelWidth stores a value within range as-is', () => {
    const store = freshSettingsStore();
    store.getState().settings.setChatPanelWidth(420);
    expect(store.getState().settings.chatPanelWidth).toBe(420);
  });

  it('setChatPanelWidth clamps values below MIN_CHAT_WIDTH up to the floor', () => {
    const store = freshSettingsStore();
    store.getState().settings.setChatPanelWidth(200);
    expect(store.getState().settings.chatPanelWidth).toBe(MIN_CHAT_WIDTH);
  });

  it('setChatPanelWidth clamps values above MAX_CHAT_WIDTH down to the ceiling', () => {
    const store = freshSettingsStore();
    store.getState().settings.setChatPanelWidth(900);
    expect(store.getState().settings.chatPanelWidth).toBe(MAX_CHAT_WIDTH);
  });

  it('setChatPanelWidth falls back to default for non-finite input', () => {
    const store = freshSettingsStore();
    store.getState().settings.setChatPanelWidth(Number.NaN);
    expect(store.getState().settings.chatPanelWidth).toBe(DEFAULT_CHAT_WIDTH);
  });
});
