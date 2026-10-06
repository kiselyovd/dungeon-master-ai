import { useCallback, useRef, useState } from 'react';
import { type AdventureActionRequest, actInAdventure } from '../../api/campaigns';
import type { AdventureAction } from '../../api/contracts/adventure';
import { ChatError } from '../../api/errors';
import { buildCampaignState } from '../../state/campaignProjection';
import { useStore } from '../../state/useStore';

interface Operation {
  campaignId: string;
  generation: number;
  request: AdventureActionRequest;
}

export function useAdventure() {
  const projection = useStore((s) => s.session.adventure);
  const generation = useStore((s) => s.session.transitionGeneration);
  const retryOperation = useRef<Operation | null>(null);
  const running = useRef<Operation | null>(null);
  const [busy, setBusy] = useState<Operation | null>(null);
  const [failure, setFailure] = useState<Operation | null>(null);
  const current = useCallback((op: Operation) => {
    const state = useStore.getState();
    return (
      state.session.transitionGeneration === op.generation &&
      state.session.activeCampaignId === op.campaignId &&
      state.session.activeSessionId === op.request.sessionId
    );
  }, []);
  const send = useCallback(
    async (op: Operation) => {
      if (!current(op) || (running.current && current(running.current))) return;
      if (!useStore.getState().session.beginMutation(op.request.causationId)) return;
      running.current = op;
      setBusy(op);
      setFailure(null);
      try {
        const confirmed = await actInAdventure(op.campaignId, op.request);
        if (!current(op)) return;
        if (
          confirmed.campaign.id !== op.campaignId ||
          confirmed.session_id !== op.request.sessionId ||
          confirmed.character_id !== op.request.characterId ||
          confirmed.campaign.revision <= op.request.expectedRevision
        ) {
          throw new ChatError('invalid_response', 'adventure action identity is invalid');
        }
        // A concurrent checkpoint may already have advanced metadata. Never apply
        // an older receipt as a replacement for a newer projection.
        if (confirmed.campaign.revision < (useStore.getState().campaigns.active?.revision ?? 0)) {
          throw new ChatError('invalid_response', 'adventure receipt is obsolete');
        }
        const restored = buildCampaignState(confirmed);
        retryOperation.current = null;
        useStore.getState().restoration.apply(restored);
      } catch (error) {
        if (!current(op)) return;
        retryOperation.current =
          error instanceof ChatError && error.code === 'http_error' ? null : op;
        setFailure(op);
      } finally {
        if (retryOperation.current !== op || !current(op))
          useStore.getState().session.endMutation(op.request.causationId);
        if (running.current === op) {
          running.current = null;
          setBusy(null);
        }
      }
    },
    [current],
  );
  const perform = useCallback(
    async (action: AdventureAction) => {
      const state = useStore.getState();
      const campaign = state.campaigns.active;
      const sessionId = state.session.activeSessionId;
      const characterId = state.pc.characterId;
      if (!campaign || !sessionId || !characterId || state.chat.isStreaming || state.combat.active)
        return;
      const pending = retryOperation.current;
      if (pending && current(pending)) return;
      const op: Operation = {
        campaignId: campaign.id,
        generation: state.session.transitionGeneration,
        request: {
          sessionId,
          characterId,
          expectedRevision: campaign.revision,
          causationId: `adventure-${crypto.randomUUID()}`,
          action,
        },
      };
      await send(op);
    },
    [current, send],
  );
  const retry = useCallback(async () => {
    if (retryOperation.current) await send(retryOperation.current);
  }, [send]);
  // Scope local feedback to the active transition, just like server results.
  return {
    projection,
    perform,
    retry,
    pending: busy?.generation === generation,
    error: failure?.generation === generation,
    retryRequired: retryOperation.current?.generation === generation,
  };
}
