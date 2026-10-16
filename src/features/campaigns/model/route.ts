import type { ActiveCampaignRoute } from './types';

export type AppRoute =
  | { kind: 'campaign-home' }
  | { kind: 'campaign-create'; draftId: string }
  | { kind: 'character-create'; campaignId: string }
  | { kind: 'world-preparation'; campaignId: string }
  | { kind: 'game-table'; campaignId: string; sessionId: string };

export interface AppRouteInput {
  hydrated: boolean;
  onboardingComplete: boolean;
  active: ActiveCampaignRoute | null;
}

export function resolveAppRoute(input: AppRouteInput): AppRoute {
  if (!input.hydrated || !input.onboardingComplete || input.active === null) {
    return { kind: 'campaign-home' };
  }
  if (!input.active.heroReady) {
    return { kind: 'character-create', campaignId: input.active.id };
  }
  if (!input.active.worldReady) {
    return { kind: 'world-preparation', campaignId: input.active.id };
  }
  return {
    kind: 'game-table',
    campaignId: input.active.id,
    sessionId: input.active.sessionId,
  };
}
