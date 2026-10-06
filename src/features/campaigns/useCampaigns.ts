import { useCallback, useRef } from 'react';
import {
  commitCampaignCharacter,
  createCampaign as createCampaignRequest,
  fetchCampaignState,
  fetchCampaigns,
} from '../../api/campaigns';
import type { CreateCampaignInput } from '../../api/contracts/campaign';
import type { CommitCharacterRequest } from '../../api/contracts/campaignState';
import { ChatError } from '../../api/errors';
import { buildCampaignState } from '../../state/campaignProjection';
import { PC_DATA_KEYS, type PcData } from '../../state/pc';
import { preserveLegacyCharacter } from '../../state/persistStorage';
import { useStore } from '../../state/useStore';

export function useCampaigns() {
  const campaigns = useStore((state) => state.campaigns.list);
  const active = useStore((state) => state.campaigns.active);
  const loading = useStore((state) => state.campaigns.loading);
  const errorCode = useStore((state) => state.campaigns.errorCode);
  const controller = useRef<AbortController | null>(null);
  const characterRetry = useRef<{
    campaignId: string;
    document: string;
    request: CommitCharacterRequest;
  } | null>(null);
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
            state.character_id === null &&
            before.session.activeCampaignId === campaignId &&
            before.session.activeSessionId === state.session_id &&
            before.pc.heroClass === null,
        });
        characterRetry.current = null;
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
        const campaign = await createCampaignRequest(input);
        if (!current(operation.generation)) return null;
        const state = await fetchCampaignState(campaign.id, operation.signal);
        if (!current(operation.generation)) return null;
        if (state.campaign.id !== campaign.id)
          throw new ChatError('invalid_response', 'campaign identity mismatch');
        useStore.getState().restoration.apply(buildCampaignState(state));
        const store = useStore.getState();
        store.campaigns.replaceList([
          state.campaign,
          ...store.campaigns.list.filter((item) => item.id !== campaign.id),
        ]);
        characterRetry.current = null;
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
      const document = JSON.stringify(character);
      const existing = characterRetry.current;
      const request =
        existing?.campaignId === active.id && existing.document === document
          ? existing.request
          : {
              sessionId: active.last_session_id,
              characterId: useStore.getState().pc.characterId ?? crypto.randomUUID(),
              expectedRevision: active.revision,
              causationId: `character-${crypto.randomUUID()}`,
              character,
            };
      characterRetry.current = { campaignId: active.id, document, request };
      const operation = begin();
      try {
        const state = await commitCampaignCharacter(active.id, request, operation.signal);
        if (!current(operation.generation)) throw new ChatError('aborted', 'aborted');
        if (
          state.campaign.id !== active.id ||
          state.character_id !== request.characterId ||
          state.session_id !== request.sessionId
        )
          throw new ChatError('invalid_response', 'character identity mismatch');
        useStore.getState().restoration.apply(buildCampaignState(state));
        characterRetry.current = null;
      } finally {
        if (current(operation.generation)) useStore.getState().campaigns.setLoading(false);
      }
    },
    [begin, current],
  );

  const clear = useCallback(async () => {
    const operation = begin();
    try {
      await preserveLegacy();
      if (!current(operation.generation)) return;
      characterRetry.current = null;
      useStore.getState().restoration.clear();
    } catch (error) {
      if (current(operation.generation))
        useStore.getState().campaigns.setErrorCode(ChatError.from(error).code);
    } finally {
      if (current(operation.generation)) useStore.getState().campaigns.setLoading(false);
    }
  }, [begin, current, preserveLegacy]);
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
