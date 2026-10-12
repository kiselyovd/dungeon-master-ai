const CAMPAIGN_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
export const MAX_CAMPAIGN_VISITS = 50;

/** Device-local navigation history. It cannot advance a gameplay revision. */
export function campaignVisits(value: unknown): Record<string, string> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(
    Object.entries(value)
      .filter(
        ([id, date]) =>
          CAMPAIGN_ID.test(id) && typeof date === 'string' && Number.isFinite(Date.parse(date)),
      )
      .sort((a, b) => Date.parse(b[1] as string) - Date.parse(a[1] as string))
      .slice(0, MAX_CAMPAIGN_VISITS),
  );
}

export function recordCampaignVisit(
  previous: Record<string, string>,
  id: string,
): Record<string, string> {
  if (!CAMPAIGN_ID.test(id)) return previous;
  return campaignVisits({ ...previous, [id]: new Date().toISOString() });
}
