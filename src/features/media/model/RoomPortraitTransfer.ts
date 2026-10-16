import {
  type MediaScope,
  type OwnedMediaReference,
  parseMediaReference,
} from '../../../api/contracts/mediaAsset';
import type { PendingRoomAction, RoomAction } from '../../../api/contracts/roomOperations';

interface Ports {
  read(reference: OwnedMediaReference, scope: MediaScope): Promise<Blob>;
  encode(bytes: Uint8Array): string;
  upload(action: Extract<RoomAction, { kind: 'upload_portrait' }>): Promise<unknown>;
}
export class RoomPortraitTransfer {
  private uploaded: { localId: string; roomId: string; reference: OwnedMediaReference } | null =
    null;
  constructor(private readonly ports: Ports) {}
  async prepare(
    character: Record<string, unknown>,
    roomId: string,
    draftId: string | null,
    pending: PendingRoomAction | null,
  ) {
    const prepared = { ...character };
    if (!character.portraitMediaRef) return prepared;
    const reference = parseMediaReference(character.portraitMediaRef);
    if (reference.source !== 'owned') return prepared;
    let remote: OwnedMediaReference;
    if (pending?.kind === 'submit_character' && pending.character.portraitMediaRef) {
      const retained = parseMediaReference(pending.character.portraitMediaRef);
      if (
        retained.source !== 'owned' ||
        retained.descriptor.sha256 !== reference.descriptor.sha256 ||
        retained.category !== 'portrait'
      )
        throw new Error('native_room_action_pending');
      remote = retained;
    } else if (this.uploaded?.localId === reference.asset_id && this.uploaded.roomId === roomId)
      remote = this.uploaded.reference;
    else {
      if (!draftId) throw new Error('media_asset_missing');
      const blob = await this.ports.read(reference, { kind: 'draft', draft_id: draftId });
      const action = {
        kind: 'upload_portrait' as const,
        asset_id: pending?.kind === 'upload_portrait' ? pending.asset_id : crypto.randomUUID(),
        mime_type: reference.descriptor.mime_type,
        data_b64: this.ports.encode(new Uint8Array(await blob.arrayBuffer())),
      };
      const uploaded = parseMediaReference(await this.ports.upload(action));
      if (
        uploaded.source !== 'owned' ||
        uploaded.descriptor.sha256 !== reference.descriptor.sha256 ||
        uploaded.category !== 'portrait'
      )
        throw new Error('media_asset_corrupt');
      remote = uploaded;
      this.uploaded = { localId: reference.asset_id, roomId, reference: remote };
    }
    prepared.portraitMediaRef = remote;
    prepared.portraitUrl = null;
    return prepared;
  }
}
