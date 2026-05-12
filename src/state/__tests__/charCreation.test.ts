import { describe, expect, it } from 'vitest';
import { createStore } from 'zustand/vanilla';
import { type CharCreationSlice, createCharCreationSlice, EMPTY_DRAFT } from '../charCreation';

function buildStore() {
  return createStore<CharCreationSlice>()((...a) => ({ ...createCharCreationSlice(...a) }));
}

describe('charCreation slice', () => {
  it('starts with empty draft and class as active tab', () => {
    const s = buildStore().getState().charCreation;
    expect(s.classId).toBeNull();
    expect(s.activeTab).toBe('class');
    expect(s.isAssisting).toBe(false);
  });

  it('setDraftField updates one field', () => {
    const store = buildStore();
    store.getState().charCreation.setDraftField('classId', 'fighter');
    expect(store.getState().charCreation.classId).toBe('fighter');
  });

  it('setActiveTab changes activeTab', () => {
    const store = buildStore();
    store.getState().charCreation.setActiveTab('race');
    expect(store.getState().charCreation.activeTab).toBe('race');
  });

  it('setAbilityScore updates one ability', () => {
    const store = buildStore();
    store.getState().charCreation.setAbilityScore('str', 15);
    expect(store.getState().charCreation.abilities.str).toBe(15);
  });

  it('rollAbilityScores appends 6-int array to history', () => {
    const store = buildStore();
    store.getState().charCreation.rollAbilityScores();
    expect(store.getState().charCreation.abilityRollHistory).toHaveLength(1);
    const firstRoll = store.getState().charCreation.abilityRollHistory[0];
    expect(firstRoll).toHaveLength(6);
    firstRoll?.forEach((v) => {
      expect(v).toBeGreaterThanOrEqual(3);
      expect(v).toBeLessThanOrEqual(18);
    });
  });

  it('rollAbilityScores caps at 3 attempts', () => {
    const store = buildStore();
    store.getState().charCreation.rollAbilityScores();
    store.getState().charCreation.rollAbilityScores();
    store.getState().charCreation.rollAbilityScores();
    store.getState().charCreation.rollAbilityScores();
    expect(store.getState().charCreation.abilityRollHistory).toHaveLength(3);
  });

  it('applyAiSuggestion merges patch into draft', () => {
    const store = buildStore();
    store.getState().charCreation.applyAiSuggestion({ classId: 'wizard', raceId: 'elf' });
    const s = store.getState().charCreation;
    expect(s.classId).toBe('wizard');
    expect(s.raceId).toBe('elf');
  });

  it('resetDraft restores empty defaults', () => {
    const store = buildStore();
    store.getState().charCreation.setDraftField('classId', 'fighter');
    store.getState().charCreation.resetDraft();
    expect(store.getState().charCreation.classId).toBeNull();
  });

  it('EMPTY_DRAFT exports a usable initial value', () => {
    expect(EMPTY_DRAFT.classId).toBeNull();
    expect(EMPTY_DRAFT.abilities.str).toBe(10);
  });
});
