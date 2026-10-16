import { describe, expect, it } from 'vitest';
import { resolveAppRoute } from '../route';

const campaignWithoutHero = {
  id: '62ba8579-904b-4460-a68a-df6268ed404a',
  sessionId: 'b5271021-b82c-40c4-aa9f-554af0d79b22',
  heroReady: false,
  worldReady: false,
};

describe('resolveAppRoute', () => {
  it('routes a hydrated user without an active campaign to campaign home', () => {
    expect(resolveAppRoute({ hydrated: true, onboardingComplete: true, active: null })).toEqual({
      kind: 'campaign-home',
    });
  });

  it('routes a campaign without a hero to character creation', () => {
    expect(
      resolveAppRoute({
        hydrated: true,
        onboardingComplete: true,
        active: campaignWithoutHero,
      }),
    ).toEqual({ kind: 'character-create', campaignId: campaignWithoutHero.id });
  });

  it('routes through preparation before the game table', () => {
    expect(
      resolveAppRoute({
        hydrated: true,
        onboardingComplete: true,
        active: { ...campaignWithoutHero, heroReady: true },
      }),
    ).toEqual({ kind: 'world-preparation', campaignId: campaignWithoutHero.id });
    expect(
      resolveAppRoute({
        hydrated: true,
        onboardingComplete: true,
        active: { ...campaignWithoutHero, heroReady: true, worldReady: true },
      }),
    ).toEqual({
      kind: 'game-table',
      campaignId: campaignWithoutHero.id,
      sessionId: campaignWithoutHero.sessionId,
    });
  });
});
