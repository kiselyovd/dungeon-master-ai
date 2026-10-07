/** Composition boundary: refresh only the local campaign before exposing its UI. */
export async function exitRoomToLocalCampaign(
  localCampaignId: string | null,
  activate: (id: string) => Promise<unknown | null>,
  onExit: () => void,
): Promise<void> {
  if (localCampaignId && !(await activate(localCampaignId)))
    throw new Error('local_campaign_recovery_failed');
  onExit();
}
