import type { StateCreator } from 'zustand';
import type { Campaign, CampaignSummary } from '../api/contracts/campaign';

export interface CampaignsData {
  list: CampaignSummary[];
  active: Campaign | null;
  loading: boolean;
  errorCode: string | null;
}

export interface CampaignsActions {
  replaceList: (campaigns: CampaignSummary[]) => void;
  setActive: (campaign: Campaign | null) => void;
  setLoading: (loading: boolean) => void;
  setErrorCode: (errorCode: string | null) => void;
}

export interface CampaignsSlice {
  campaigns: CampaignsData & CampaignsActions;
}

export const createCampaignsSlice: StateCreator<CampaignsSlice, [], [], CampaignsSlice> = (
  set,
) => ({
  campaigns: {
    list: [],
    active: null,
    loading: false,
    errorCode: null,
    replaceList: (list) => set((state) => ({ campaigns: { ...state.campaigns, list } })),
    setActive: (active) => set((state) => ({ campaigns: { ...state.campaigns, active } })),
    setLoading: (loading) => set((state) => ({ campaigns: { ...state.campaigns, loading } })),
    setErrorCode: (errorCode) => set((state) => ({ campaigns: { ...state.campaigns, errorCode } })),
  },
});
