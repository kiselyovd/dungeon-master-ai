import * as v from 'valibot';
import { type RestRequest, RestRequestSchema } from '../api/contracts/rest';

const key = (campaignId: string) => `dmai.rest-outbox.v1.${campaignId}`;
export function loadRestOutbox(campaignId: string): RestRequest | null {
  let raw: string | null;
  try {
    raw = localStorage.getItem(key(campaignId));
  } catch {
    return null;
  }
  if (!raw) return null;
  try {
    const parsed = v.safeParse(RestRequestSchema, JSON.parse(raw));
    if (parsed.success) return parsed.output;
  } catch {}
  try {
    localStorage.removeItem(key(campaignId));
  } catch {}
  return null;
}
export function saveRestOutbox(campaignId: string, request: RestRequest) {
  localStorage.setItem(key(campaignId), JSON.stringify(v.parse(RestRequestSchema, request)));
}
export function clearRestOutbox(campaignId: string) {
  localStorage.removeItem(key(campaignId));
}
