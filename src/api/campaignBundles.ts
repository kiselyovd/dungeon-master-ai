import { invoke, isTauri } from '@tauri-apps/api/core';
import * as v from 'valibot';

const uuid = v.pipe(v.string(), v.uuid());
const kind = v.picklist(['full_campaign_backup', 'player_dossier']);
const preview = v.strictObject({
  kind,
  title: v.pipe(v.string(), v.maxLength(200)),
  character_count: v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(4)),
  message_count: v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(20_000)),
  media_count: v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(64)),
  private_backup: v.boolean(),
});
const choice = v.strictObject({
  operationId: uuid,
  preview,
  fileName: v.pipe(v.string(), v.maxLength(512)),
});
const result = v.strictObject({
  campaign_id: uuid,
  session_id: uuid,
  revision: v.pipe(v.number(), v.integer(), v.minValue(0)),
  remapped_ids: v.record(uuid, uuid),
});
const exported = v.strictObject({
  fileName: v.pipe(v.string(), v.maxLength(512)),
  bytes: v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(64 * 1024 * 1024)),
});
export type BundleKind = v.InferOutput<typeof kind>;
export type BundleImportChoice = v.InferOutput<typeof choice>;
export type BundleImportResult = v.InferOutput<typeof result>;
export type BundleExportResult = v.InferOutput<typeof exported>;
export function nativeBundlesAvailable(): boolean {
  return isTauri();
}
function native(): void {
  if (!isTauri()) throw Error('native_bundle_unavailable');
}
export async function pickCampaignImport(
  language: 'en' | 'ru',
): Promise<BundleImportChoice | null> {
  native();
  const value = await invoke('desktop_bundle_pick_import', { language });
  return value === null ? null : v.parse(choice, value);
}
export async function confirmCampaignImport(operationId: string): Promise<BundleImportResult> {
  native();
  return v.parse(result, await invoke('desktop_bundle_confirm_import', { operationId }));
}
export async function cancelCampaignImport(operationId: string): Promise<void> {
  native();
  await invoke('desktop_bundle_cancel', { operationId });
}
export async function exportCampaignFile(
  campaignId: string,
  kind: BundleKind,
  language: 'en' | 'ru',
): Promise<BundleExportResult | null> {
  native();
  const value = await invoke('desktop_bundle_export', { campaignId, kind, language });
  return value === null ? null : v.parse(exported, value);
}
export async function revealCampaignFile(): Promise<void> {
  native();
  await invoke('desktop_bundle_reveal');
}
export function bundleFailureCode(error: unknown): string {
  const code = typeof error === 'string' ? error : error instanceof Error ? error.message : '';
  if (/^(native_bundle_|bundle_)[a-z_]+$/.test(code)) return code;
  return 'native_bundle_request_failed';
}
