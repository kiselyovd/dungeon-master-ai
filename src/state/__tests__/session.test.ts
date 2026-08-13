import { describe, expect, it } from 'vitest';
import { create } from 'zustand';
import { createSessionSlice, type SessionSlice } from '../session';

function freshStore() {
  return create<SessionSlice>()((...args) => ({
    ...createSessionSlice(...args),
  }));
}

describe('SessionSlice', () => {
  it('starts inactive and never mints IDs', () => {
    const store = freshStore();
    expect(store.getState().session.activeCampaignId).toBeNull();
    expect(store.getState().session.activeSessionId).toBeNull();
    expect(store.getState().session.current()).toBeNull();
  });

  it('activates and clears the backend-owned pair atomically', () => {
    const store = freshStore();
    store.getState().session.activate('campaign-id', 'session-id');
    expect(store.getState().session.current()).toEqual({
      campaignId: 'campaign-id',
      sessionId: 'session-id',
    });

    store.getState().session.clear();
    expect(store.getState().session.current()).toBeNull();
  });

  it('does not expose the legacy UUID-minting action', () => {
    const store = freshStore();
    expect('ensureSession' in store.getState().session).toBe(false);
  });

  it('updates scene state without changing activation', () => {
    const store = freshStore();
    store.getState().session.activate('campaign-id', 'session-id');
    store.getState().session.setCurrentScene({ name: 'Crimson Sanctuary', stepCounter: 2 });
    store.getState().session.incrementScene();
    expect(store.getState().session.currentScene).toEqual({
      name: 'Crimson Sanctuary',
      stepCounter: 3,
    });
    expect(store.getState().session.current()).toEqual({
      campaignId: 'campaign-id',
      sessionId: 'session-id',
    });
  });
});
