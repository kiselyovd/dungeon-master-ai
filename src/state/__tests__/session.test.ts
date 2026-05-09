import { describe, expect, it } from 'vitest';
import { create } from 'zustand';
import { createSessionSlice, type SessionSlice } from '../session';

function freshStore() {
  return create<SessionSlice>()((...a) => ({
    ...createSessionSlice(...a),
  }));
}

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

describe('SessionSlice', () => {
  it('starts with both IDs null', () => {
    const store = freshStore();
    expect(store.getState().session.activeCampaignId).toBeNull();
    expect(store.getState().session.activeSessionId).toBeNull();
  });

  it('ensureSession mints a UUID pair when missing', () => {
    const store = freshStore();
    const { campaignId, sessionId } = store.getState().session.ensureSession();
    expect(campaignId).toMatch(UUID_RE);
    expect(sessionId).toMatch(UUID_RE);
    expect(store.getState().session.activeCampaignId).toBe(campaignId);
    expect(store.getState().session.activeSessionId).toBe(sessionId);
  });

  it('ensureSession is idempotent once IDs exist', () => {
    const store = freshStore();
    const first = store.getState().session.ensureSession();
    const second = store.getState().session.ensureSession();
    expect(second).toEqual(first);
  });

  it('setActiveSession overrides both IDs together', () => {
    const store = freshStore();
    store.getState().session.setActiveSession('camp-1', 'sess-1');
    expect(store.getState().session.activeCampaignId).toBe('camp-1');
    expect(store.getState().session.activeSessionId).toBe('sess-1');
  });

  it('clearSession nulls both IDs and the next ensureSession mints fresh', () => {
    const store = freshStore();
    const first = store.getState().session.ensureSession();
    store.getState().session.clearSession();
    expect(store.getState().session.activeCampaignId).toBeNull();
    expect(store.getState().session.activeSessionId).toBeNull();
    const second = store.getState().session.ensureSession();
    expect(second.campaignId).not.toBe(first.campaignId);
    expect(second.sessionId).not.toBe(first.sessionId);
  });

  it('starts with currentScene null', () => {
    const store = freshStore();
    expect(store.getState().session.currentScene).toBeNull();
  });

  it('setCurrentScene sets and clears the scene snapshot', () => {
    const store = freshStore();
    store.getState().session.setCurrentScene({ name: 'Crimson Sanctuary', stepCounter: 0 });
    expect(store.getState().session.currentScene).toEqual({
      name: 'Crimson Sanctuary',
      stepCounter: 0,
    });
    store.getState().session.setCurrentScene(null);
    expect(store.getState().session.currentScene).toBeNull();
  });

  it('incrementScene bumps stepCounter when a scene is set', () => {
    const store = freshStore();
    store.getState().session.setCurrentScene({ name: 'Crimson Sanctuary', stepCounter: 2 });
    store.getState().session.incrementScene();
    store.getState().session.incrementScene();
    expect(store.getState().session.currentScene).toEqual({
      name: 'Crimson Sanctuary',
      stepCounter: 4,
    });
  });

  it('incrementScene is a no-op when no scene is set', () => {
    const store = freshStore();
    store.getState().session.incrementScene();
    expect(store.getState().session.currentScene).toBeNull();
  });
});
