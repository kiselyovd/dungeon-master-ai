import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { RestoredSessionProjection } from '../../features/saves/model/buildRestoredSession';
import { useStore } from '../useStore';

describe('atomic session restoration', () => {
  beforeEach(() => {
    useStore.getState().chat.reset();
    useStore.getState().combat.clearProjection();
    useStore.getState().session.clear();
  });

  it('applies session, messages, pc, combat, and scene in one Zustand notification', () => {
    const notifications: string[] = [];
    const unsubscribe = useStore.subscribe((state) =>
      notifications.push(state.session.activeSessionId ?? 'none'),
    );
    const projection: RestoredSessionProjection = {
      campaignId: 'campaign-1',
      sessionId: 'session-restored',
      revision: 4,
      messages: [{ id: 'm1', role: 'assistant', content: 'Restored', sequenceIndex: 0 }],
      pc: { name: 'Restored Hero', hp: 7 },
      scene: { name: 'Vault', stepCounter: 0 },
      combat: null,
      journal: [],
      npcs: [],
    };

    useStore.getState().restoration.apply(projection);
    unsubscribe();

    const state = useStore.getState();
    expect(notifications).toEqual(['session-restored']);
    expect(state.session).toMatchObject({
      activeCampaignId: 'campaign-1',
      activeSessionId: 'session-restored',
      currentScene: { name: 'Vault', stepCounter: 0 },
    });
    expect(state.chat.messages).toEqual(projection.messages);
    expect(state.pc).toMatchObject({ name: 'Restored Hero', hp: 7 });
    expect(state.combat.active).toBe(false);
  });

  it('clears the previous hero and transient save state when the next campaign is empty', () => {
    useStore.getState().pc.applyPreset('fighter');
    useStore.getState().pc.replaceFromDraft({ portraitUrl: 'https://example.test/old.png' });
    useStore.getState().saves.selectSave('old-save');
    useStore.getState().toolLog.addPending('old', 'attack', {}, 1);
    useStore.getState().restoration.apply({
      campaignId: 'other',
      sessionId: 'other-session',
      revision: 1,
      messages: [],
      pc: {},
      scene: null,
      combat: null,
      journal: [],
      npcs: [],
    });
    const state = useStore.getState();
    expect(state.pc.heroClass).toBeNull();
    expect(state.pc.inventory).toEqual([]);
    expect(state.pc.portraitUrl).toBeNull();
    expect(state.pc.skillProfs).toEqual({});
    expect(state.saves.selectedSaveId).toBeNull();
    expect(state.toolLog.entries).toEqual([]);
  });

  it('keeps live presentation continuity while a restored timeline resets it', () => {
    const projection: RestoredSessionProjection = {
      campaignId: 'campaign',
      sessionId: 'session',
      revision: 1,
      messages: [],
      pc: {},
      scene: null,
      combat: null,
      journal: [],
      npcs: [],
    };
    useStore.getState().restoration.apply(projection);
    const epoch = useStore.getState().session.presentationGeneration;
    const requestGeneration = useStore.getState().session.transitionGeneration;
    useStore.getState().restoration.apply({ ...projection, revision: 2, liveUpdate: true });
    expect(useStore.getState().session.presentationGeneration).toBe(epoch);
    expect(useStore.getState().session.transitionGeneration).toBeGreaterThan(requestGeneration);
    useStore.getState().restoration.apply({ ...projection, revision: 3 });
    expect(useStore.getState().session.presentationGeneration).toBe(epoch + 1);
  });

  it('aborts the old transport before dropping its controller', () => {
    const controller = new AbortController();
    const abort = vi.spyOn(controller, 'abort');
    useStore.setState((state) => ({ chat: { ...state.chat, abortController: controller } }));
    useStore.getState().restoration.apply({
      campaignId: 'other',
      sessionId: 'other-session',
      revision: 1,
      messages: [],
      pc: {},
      scene: null,
      combat: null,
      journal: [],
      npcs: [],
    });
    expect(abort).toHaveBeenCalledOnce();
    expect(useStore.getState().chat.abortController).toBeNull();
  });
});
