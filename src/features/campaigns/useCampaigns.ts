import { useCallback, useRef } from 'react';
import {
  commitCampaignCharacter,
  createCampaign as createCampaignRequest,
  fetchCampaignState,
  fetchCampaigns,
} from '../../api/campaigns';
import type { CreateCampaignInput } from '../../api/contracts/campaign';
import { ChatError } from '../../api/errors';
import {
  acknowledgeCampaignCharacter,
  acknowledgeCampaignCreation,
  prepareCampaignCharacter,
  prepareCampaignCreation,
} from '../../state/campaignPreparation';
import { buildCampaignState } from '../../state/campaignProjection';
import { PC_DATA_KEYS, type PcData } from '../../state/pc';
import { persistCharacterDraft, preserveLegacyCharacter } from '../../state/persistStorage';
import { useStore } from '../../state/useStore';

export function useCampaigns() {
  const campaigns = useStore((state) => state.campaigns.list);
  const active = useStore((state) => state.campaigns.active);
  const loading = useStore((state) => state.campaigns.loading);
  const errorCode = useStore((state) => state.campaigns.errorCode);
  const controller = useRef<AbortController | null>(null);
  const begin = useCallback(() => {
    controller.current?.abort();
    const next = new AbortController();
    controller.current = next;
    const state = useStore.getState();
    const generation = state.session.beginTransition();
    state.campaigns.setLoading(true);
    state.campaigns.setErrorCode(null);
    return { generation, signal: next.signal };
  }, []);
  const current = useCallback(
    (generation: number) => useStore.getState().session.transitionGeneration === generation,
    [],
  );
  const preserveLegacy = useCallback(async () => {
    const state = useStore.getState();
    if (state.pc.heroClass && !state.pc.characterId)
      await preserveLegacyCharacter(state.pc, state.session.activeCampaignId);
  }, []);

  const refresh = useCallback(async () => {
    const generation = useStore.getState().session.transitionGeneration;
    useStore.getState().campaigns.setLoading(true);
    try {
      const list = await fetchCampaigns();
      if (!current(generation)) return;
      useStore.getState().campaigns.replaceList(list);
      useStore.getState().campaigns.setErrorCode(null);
    } catch (error) {
      if (current(generation))
        useStore.getState().campaigns.setErrorCode(ChatError.from(error).code);
    } finally {
      if (current(generation)) useStore.getState().campaigns.setLoading(false);
    }
  }, [current]);

  const activate = useCallback(
    async (campaignId: string) => {
      const operation = begin();
      try {
        await preserveLegacy();
        if (!current(operation.generation)) return null;
        let state = await fetchCampaignState(campaignId, operation.signal);
        if (!current(operation.generation)) return null;
        if (state.campaign.id !== campaignId)
          throw new ChatError('invalid_response', 'campaign identity mismatch');
        const legacy = useStore.getState();
        if (
          state.character_id === null &&
          legacy.session.activeCampaignId === campaignId &&
          legacy.pc.heroClass !== null &&
          !legacy.pc.characterId
        ) {
          const character = Object.fromEntries(
            Object.entries(legacy.pc).filter(([key]) => PC_DATA_KEYS.has(key as keyof PcData)),
          );
          state = await commitCampaignCharacter(
            campaignId,
            {
              sessionId: state.session_id,
              characterId: crypto.randomUUID(),
              expectedRevision: state.campaign.revision,
              causationId: `legacy-character-${crypto.randomUUID()}`,
              character,
            },
            operation.signal,
          );
          if (!current(operation.generation)) return null;
          if (state.campaign.id !== campaignId)
            throw new ChatError('invalid_response', 'campaign identity mismatch');
        }
        const before = useStore.getState();
        useStore.getState().restoration.apply({
          ...buildCampaignState(state),
          preserveDraft:
            before.charCreation.draftScope?.kind === 'room' ||
            (before.charCreation.draftScope?.kind === 'campaign' &&
              before.charCreation.draftScope.campaignId === campaignId &&
              before.charCreation.draftScope.characterId === state.character_id) ||
            (before.session.activeCampaignId === campaignId &&
              before.session.activeSessionId === state.session_id &&
              ((state.character_id === null && before.pc.heroClass === null) ||
                (before.charCreation.draftScope?.kind === 'campaign' &&
                  before.charCreation.draftScope.campaignId === campaignId &&
                  before.charCreation.draftScope.characterId === state.character_id))),
        });
        useStore.getState().settings.recordCampaignVisit(campaignId);
        return state.campaign;
      } catch (error) {
        if (current(operation.generation))
          useStore.getState().campaigns.setErrorCode(ChatError.from(error).code);
        return null;
      } finally {
        if (current(operation.generation)) useStore.getState().campaigns.setLoading(false);
      }
    },
    [begin, current, preserveLegacy],
  );

  const create = useCallback(
    async (input: CreateCampaignInput) => {
      const operation = begin();
      try {
        await preserveLegacy();
        if (!current(operation.generation)) return null;
        const attempt = await prepareCampaignCreation(input);
        if (!current(operation.generation)) return null;
        const campaign = await createCampaignRequest(attempt);
        if (!current(operation.generation)) return null;
        const state = await fetchCampaignState(campaign.id, operation.signal);
        if (!current(operation.generation)) return null;
        if (state.campaign.id !== campaign.id)
          throw new ChatError('invalid_response', 'campaign identity mismatch');
        await acknowledgeCampaignCreation(attempt.requestId);
        if (!current(operation.generation)) return null;
        useStore.getState().restoration.apply(buildCampaignState(state));
        const store = useStore.getState();
        store.campaigns.replaceList([
          state.campaign,
          ...store.campaigns.list.filter((item) => item.id !== campaign.id),
        ]);
        return state.campaign;
      } catch (error) {
        if (current(operation.generation))
          useStore.getState().campaigns.setErrorCode(ChatError.from(error).code);
        return null;
      } finally {
        if (current(operation.generation)) useStore.getState().campaigns.setLoading(false);
      }
    },
    [begin, current, preserveLegacy],
  );

  const commitCharacter = useCallback(
    async (character: Record<string, unknown>) => {
      const active = useStore.getState().campaigns.active;
      if (!active) throw new ChatError('campaign_required', 'campaign_required');
      const operation = begin();
      try {
        await persistCharacterDraft(useStore.getState().charCreation);
        if (!current(operation.generation)) throw new ChatError('aborted', 'aborted');
        const canonical = await fetchCampaignState(active.id, operation.signal);
        if (!current(operation.generation)) throw new ChatError('aborted', 'aborted');
        if (canonical.campaign.id !== active.id || canonical.session_id !== active.last_session_id)
          throw new ChatError('invalid_response', 'character scope mismatch');
        const request = await prepareCampaignCharacter(active.id, {
          sessionId: active.last_session_id,
          characterId: canonical.character_id,
          expectedRevision: canonical.campaign.revision,
          character,
        });
        if (!current(operation.generation)) throw new ChatError('aborted', 'aborted');
        const state = await commitCampaignCharacter(active.id, request, operation.signal);
        if (!current(operation.generation)) throw new ChatError('aborted', 'aborted');
        if (
          state.campaign.id !== active.id ||
          state.character_id !== request.characterId ||
          state.session_id !== request.sessionId
        )
          throw new ChatError('invalid_response', 'character identity mismatch');
        await acknowledgeCampaignCharacter(active.id, request.causationId);
        if (!current(operation.generation)) throw new ChatError('aborted', 'aborted');
        useStore.getState().restoration.apply(buildCampaignState(state));
      } finally {
        if (current(operation.generation)) useStore.getState().campaigns.setLoading(false);
      }
    },
    [begin, current],
  );

  const clear = useCallback(
    async (preserveDraft = false) => {
      const operation = begin();
      try {
        await preserveLegacy();
        if (!current(operation.generation)) return;
        useStore.getState().restoration.clear(preserveDraft);
      } catch (error) {
        if (current(operation.generation))
          useStore.getState().campaigns.setErrorCode(ChatError.from(error).code);
      } finally {
        if (current(operation.generation)) useStore.getState().campaigns.setLoading(false);
      }
    },
    [begin, current, preserveLegacy],
  );
  return {
    campaigns,
    active,
    loading,
    errorCode,
    refresh,
    activate,
    create,
    clear,
    commitCharacter,
  };
}
