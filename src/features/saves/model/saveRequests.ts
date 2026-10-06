import { ChatError } from '../../../api/errors';
import type { CreateSaveRequest, SaveCreated, SnapshotContext } from '../../../api/saves';

export interface PendingSave {
  kind: 'quick' | 'manual';
  sessionId: string;
  context: SnapshotContext;
  body?: CreateSaveRequest;
  signature: string;
}

interface WriteIntent {
  kind: PendingSave['kind'];
  campaignId: string;
  sessionId: string;
  character: unknown;
  body?: CreateSaveRequest;
  current(): boolean;
  revision(): number;
  reconcile(result: SaveCreated): void;
}

/** One queue per campaign/session, shared by the hotkey and saves-screen hooks.
 * Ambiguous transport failures retain the exact original request until acknowledged. */
export class SaveRequests {
  private readonly tails = new Map<string, Promise<unknown>>();
  private readonly pending = new Map<string, PendingSave>();

  constructor(private readonly send: (request: PendingSave) => Promise<SaveCreated>) {}

  async write(intent: WriteIntent): Promise<SaveCreated | null> {
    const scope = `${intent.campaignId}/${intent.sessionId}`;
    return this.serialize(scope, async () => {
      if (!intent.current()) return null;
      const signature = JSON.stringify({
        kind: intent.kind,
        character: intent.character,
        body: intent.body,
      });
      for (const [key, pending] of this.pending) {
        if (
          pending.context.campaignId !== intent.campaignId ||
          pending.sessionId !== intent.sessionId
        )
          continue;
        const result = await this.perform(key, pending);
        if (!intent.current()) return null;
        intent.reconcile(result);
        if (pending.signature === signature) return result;
      }
      const key = `${scope}/${intent.kind}`;
      const request: PendingSave = {
        kind: intent.kind,
        sessionId: intent.sessionId,
        signature,
        context: {
          campaignId: intent.campaignId,
          expectedRevision: intent.revision(),
          character: intent.character,
          causationId: `save-${crypto.randomUUID()}`,
        },
        ...(intent.body ? { body: intent.body } : {}),
      };
      this.pending.set(key, request);
      const result = await this.perform(key, request);
      if (!intent.current()) return null;
      intent.reconcile(result);
      return result;
    });
  }

  async serialize<T>(scope: string, work: () => Promise<T>): Promise<T> {
    const previous = this.tails.get(scope) ?? Promise.resolve();
    const operation = previous.catch(() => undefined).then(work);
    this.tails.set(scope, operation);
    try {
      return await operation;
    } finally {
      if (this.tails.get(scope) === operation) this.tails.delete(scope);
    }
  }

  private async perform(key: string, request: PendingSave): Promise<SaveCreated> {
    try {
      const result = await this.send(request);
      this.pending.delete(key);
      return result;
    } catch (error) {
      // An explicit HTTP rejection did not create a checkpoint. A lost or malformed
      // response may have followed a commit, so preserve that causation identity.
      if (error instanceof ChatError && error.code === 'http_error') this.pending.delete(key);
      throw error;
    }
  }
}
