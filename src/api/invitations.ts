import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import * as v from 'valibot';

const previewSchema = v.strictObject({
  address: v.pipe(v.string(), v.maxLength(96)),
  room_id: v.pipe(v.string(), v.uuid()),
  invitation: v.pipe(v.string(), v.regex(/^[a-fA-F0-9]{64}$/)),
});
const eventSchema = v.strictObject({
  event_id: v.pipe(v.string(), v.uuid()),
  preview: previewSchema,
});
export type InvitationPreview = v.InferOutput<typeof previewSchema>;
export type InvitationEvent = v.InferOutput<typeof eventSchema>;

/** Browser preview mirrors the grammar; Tauri always validates again natively. */
export function parseBrowserInvitation(raw: string): InvitationPreview {
  const invalid = () => Error('native_invitation_invalid');
  if (raw.length > 1024 || /\s/.test(raw)) throw invalid();
  let uri: URL;
  try {
    uri = new URL(raw);
  } catch {
    throw invalid();
  }
  if (
    raw.split('?')[0] !== 'dmai://join/v1' ||
    raw.includes('#') ||
    uri.port ||
    uri.username ||
    uri.password
  )
    throw invalid();
  const values = new Map<string, string>();
  for (const [key, value] of uri.searchParams) {
    if (!['address', 'room', 'invitation'].includes(key) || values.has(key)) throw invalid();
    values.set(key, value);
  }
  const result = v.safeParse(previewSchema, {
    address: values.get('address'),
    room_id: values.get('room'),
    invitation: values.get('invitation'),
  });
  if (!result.success) throw invalid();
  const address = result.output.address;
  const ipv4 =
    /^(0|[1-9]\d{0,2})\.(0|[1-9]\d{0,2})\.(0|[1-9]\d{0,2})\.(0|[1-9]\d{0,2}):(\d{1,5})$/.exec(
      address,
    );
  const ipv6 = /^\[([0-9a-fA-F:.]+)\]:(\d{1,5})$/.exec(address);
  const port = Number(ipv4?.[5] ?? ipv6?.[2] ?? 0);
  if (port < 1 || port > 65535) throw invalid();
  if (ipv4) {
    const octets = ipv4.slice(1, 5).map(Number);
    if (
      octets.some((octet) => octet > 255) ||
      octets.every((octet) => octet === 0) ||
      (octets[0] ?? 0) >= 224
    )
      throw invalid();
  } else if (ipv6) {
    let normalized: string;
    try {
      normalized = new URL(`http://${address}`).hostname;
    } catch {
      throw invalid();
    }
    if (normalized === '[::]' || normalized.toLowerCase().startsWith('[ff')) throw invalid();
    const mapped = /^\[::(?:ffff:)?([0-9a-f]{1,4}):([0-9a-f]{1,4})\]$/i.exec(normalized);
    if (mapped) {
      const high = Number.parseInt(mapped[1] ?? '0', 16);
      const low = Number.parseInt(mapped[2] ?? '0', 16);
      if ((high === 0 && low === 0) || high >>> 8 >= 224) throw invalid();
    }
  } else throw invalid();
  if (result.output.room_id === '00000000-0000-0000-0000-000000000000') throw invalid();
  return result.output;
}

export async function parseInvitation(raw: string): Promise<InvitationPreview> {
  if (!isTauri()) return parseBrowserInvitation(raw);
  return v.parse(previewSchema, await invoke('desktop_parse_invitation', { raw }));
}
export async function createInvitation(preview: InvitationPreview): Promise<string> {
  if (isTauri()) {
    const result = await invoke<unknown>('desktop_create_invitation', { preview });
    if (typeof result !== 'string' || result.length > 1024)
      throw Error('native_invitation_invalid');
    return result;
  }
  const uri = new URL('dmai://join/v1');
  uri.searchParams.set('address', preview.address);
  uri.searchParams.set('room', preview.room_id);
  uri.searchParams.set('invitation', preview.invitation);
  parseBrowserInvitation(uri.toString());
  return uri.toString();
}
export async function takeInvitation(): Promise<InvitationEvent | null> {
  if (!isTauri()) return null;
  const result = await invoke<unknown>('desktop_take_invitation');
  return result === null ? null : v.parse(eventSchema, result);
}
export async function listenForInvitations(notify: () => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  const window = getCurrentWindow();
  if (window.label !== 'main') return () => {};
  return window.listen('desktop-invitation-ready', notify);
}
export async function registerInvitationProtocol(): Promise<void> {
  await invoke('desktop_register_invitation_protocol');
}
export function canRegisterInvitationProtocol(): boolean {
  return isTauri();
}
