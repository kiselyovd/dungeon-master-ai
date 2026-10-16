import { useCallback } from 'react';
import {
  createCampaign as createCampaignRequest,
  fetchCampaign,
  fetchCampaigns,
} from '../../api/campaigns';
import type { CreateCampaignInput } from '../../api/contracts/campaign';
import { ChatError } from '../../api/errors';
import { useStore } from '../../state/useStore';

export function useCampaigns() {
  const campaigns = useStore((state) => state.campaigns.list);
  const active = useStore((state) => state.campaigns.active);
  const loading = useStore((state) => state.campaigns.loading);
  const errorCode = useStore((state) => state.campaigns.errorCode);

  const refresh = useCallback(async () => {
    const state = useStore.getState();
    state.campaigns.setLoading(true);
    try {
      state.campaigns.replaceList(await fetchCampaigns());
      state.campaigns.setErrorCode(null);
    } catch (error) {
      state.campaigns.setErrorCode(ChatError.from(error).code);
    } finally {
      state.campaigns.setLoading(false);
    }
  }, []);

  const activate = useCallback(async (campaignId: string) => {
    const state = useStore.getState();
    state.campaigns.setLoading(true);
    try {
      const campaign = await fetchCampaign(campaignId);
      state.campaigns.setActive(campaign);
      state.session.activate(campaign.id, campaign.last_session_id);
      state.campaigns.setErrorCode(null);
      return campaign;
    } catch (error) {
      state.campaigns.setErrorCode(ChatError.from(error).code);
      return null;
    } finally {
      state.campaigns.setLoading(false);
    }
  }, []);

  const create = useCallback(async (input: CreateCampaignInput) => {
    const state = useStore.getState();
    state.campaigns.setLoading(true);
    try {
      const campaign = await createCampaignRequest(input);
      state.campaigns.setActive(campaign);
      state.campaigns.replaceList([campaign, ...state.campaigns.list]);
      state.session.activate(campaign.id, campaign.last_session_id);
      state.campaigns.setErrorCode(null);
      return campaign;
    } catch (error) {
      state.campaigns.setErrorCode(ChatError.from(error).code);
      return null;
    } finally {
      state.campaigns.setLoading(false);
    }
  }, []);

  const clear = useCallback(() => {
    const state = useStore.getState();
    state.campaigns.setActive(null);
    state.session.clear();
  }, []);

  return { campaigns, active, loading, errorCode, refresh, activate, create, clear };
}
