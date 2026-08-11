import { beforeEach, describe, expect, it } from 'vitest';
import type { RestoredSessionProjection } from '../../features/saves/model/buildRestoredSession';
import { useStore } from '../useStore';

describe('atomic session restoration', () => {
  beforeEach(() => {
    useStore.getState().chat.reset();
    useStore.getState().combat.clearProjection();
    useStore.getState().session.clearSession();
  });

  it('applies session, messages, pc, combat, and scene in one Zustand notification', () => {
    const notifications: string[] = [];
    const unsubscribe = useStore.subscribe((state) =>
      notifications.push(state.session.activeSessionId ?? 'none'),
    );
    const projection: RestoredSessionProjection = {
      campaignId: 'campaign-1',
      sessionId: 'session-restored',
      messages: [{ id: 'm1', role: 'assistant', content: 'Restored', sequenceIndex: 0 }],
      pc: { name: 'Restored Hero', hp: 7 },
      scene: { name: 'Vault', stepCounter: 0 },
      combat: null,
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
});
