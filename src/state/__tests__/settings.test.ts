import { describe, expect, it } from 'vitest';
import { create } from 'zustand';
import { createSettingsSlice, type SettingsSlice } from '../settings';

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
