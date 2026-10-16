import type {
  Campaign,
  CampaignSummary,
  CreateCampaignInput,
} from '../../../api/contracts/campaign';

export type { Campaign, CampaignSummary, CreateCampaignInput };

export interface ActiveCampaignRoute {
  id: string;
  sessionId: string;
  heroReady: boolean;
  worldReady: boolean;
}
