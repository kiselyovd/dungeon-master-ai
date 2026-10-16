import * as v from 'valibot';
import { type SoloRecoveryRequest, SoloRecoveryRequestSchema } from '../api/contracts/soloRecovery';

const key = (campaignId: string) => `dmai.solo-recovery-outbox.v1.${campaignId}`;
export function loadSoloRecoveryOutbox(campaignId: string): SoloRecoveryRequest | null {
  let raw: string | null;
  try {
    raw = localStorage.getItem(key(campaignId));
  } catch {
    return null;
  }
  if (!raw) return null;
  try {
    const parsed = v.safeParse(SoloRecoveryRequestSchema, JSON.parse(raw));
    if (parsed.success) return parsed.output;
  } catch {}
  try {
    localStorage.removeItem(key(campaignId));
  } catch {}
  return null;
}
export function saveSoloRecoveryOutbox(campaignId: string, request: SoloRecoveryRequest) {
  localStorage.setItem(
    key(campaignId),
    JSON.stringify(v.parse(SoloRecoveryRequestSchema, request)),
  );
}
export function clearSoloRecoveryOutbox(campaignId: string) {
  localStorage.removeItem(key(campaignId));
}
