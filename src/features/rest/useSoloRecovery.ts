import { useCallback, useEffect, useRef, useState } from 'react';
import * as v from 'valibot';
import type {
  SoloRecoveryIntent,
  SoloRecoveryRequest,
  SoloRecoveryStatus,
} from '../../api/contracts/soloRecovery';
import {
  SoloRecoveryIntentSchema,
  type SoloRecoveryResult,
} from '../../api/contracts/soloRecovery';
import { ChatError } from '../../api/errors';
import { getSoloRecoveryStatus, performSoloRecovery } from '../../api/soloRecovery';
import { applyRestOutcome } from '../../state/restOutcome';
import {
  clearSoloRecoveryOutbox,
  loadSoloRecoveryOutbox,
  saveSoloRecoveryOutbox,
} from '../../state/soloRecoveryOutbox';
import { useStore } from '../../state/useStore';

export function useSoloRecovery() {
  const campaignId = useStore((s) => s.session.activeCampaignId);
  const sessionId = useStore((s) => s.session.activeSessionId);
  const characterId = useStore((s) => s.pc.characterId);
  const revision = useStore((s) => s.campaigns.active?.revision ?? 0);
  const generation = useStore((s) => s.session.transitionGeneration);
  const [status, setStatus] = useState<SoloRecoveryStatus | null>(null);
  const [receipt, setReceipt] = useState<SoloRecoveryResult['receipt'] | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const initial = campaignId ? loadSoloRecoveryOutbox(campaignId) : null;
  const pending = useRef<SoloRecoveryRequest | null>(initial);
  const running = useRef(false);
  const [retryRequired, setRetryRequired] = useState(Boolean(initial));
  const current = useCallback(() => {
    const state = useStore.getState();
    return (
      state.session.activeCampaignId === campaignId &&
      state.session.activeSessionId === sessionId &&
      state.pc.characterId === characterId &&
      state.session.transitionGeneration === generation
    );
  }, [campaignId, sessionId, characterId, generation]);
  const load = useCallback(
    async (signal?: AbortSignal) => {
      if (!campaignId || !sessionId || !characterId) return;
      setLoading(true);
      try {
        const value = await getSoloRecoveryStatus(campaignId, signal);
        if (!current()) return;
        if (
          value.campaign_id !== campaignId ||
          value.session_id !== sessionId ||
          value.character_id !== characterId
        )
          throw new ChatError('invalid_response', 'recovery_identity_mismatch');
        if (value.revision < (useStore.getState().campaigns.active?.revision ?? 0)) return;
        setStatus((previous) =>
          previous && previous.revision > value.revision ? previous : value,
        );
        applyRestOutcome(value);
      } catch (cause) {
        if (current() && !signal?.aborted) setError(ChatError.from(cause).message);
      } finally {
        if (current()) setLoading(false);
      }
    },
    [campaignId, sessionId, characterId, current],
  );
  useEffect(() => {
    const abort = new AbortController();
    if (
      pending.current &&
      (pending.current.session_id !== sessionId || pending.current.character_id !== characterId)
    ) {
      if (campaignId) clearSoloRecoveryOutbox(campaignId);
      pending.current = null;
      setRetryRequired(false);
    }
    void load(abort.signal);
    return () => abort.abort();
  }, [campaignId, sessionId, characterId, load]);
  useEffect(() => {
    if (status && revision > status.revision) void load();
  }, [revision, status, load]);
  const send = useCallback(
    async (input: SoloRecoveryRequest): Promise<boolean> => {
      if (!campaignId || !current() || running.current) return false;
      const state = useStore.getState();
      if (
        state.chat.isStreaming ||
        state.combat.active ||
        !state.session.beginMutation(input.causation_id)
      )
        return false;
      running.current = true;
      setBusy(true);
      setError(null);
      try {
        saveSoloRecoveryOutbox(campaignId, input);
        pending.current = structuredClone(input);
        setRetryRequired(true);
        const result = await performSoloRecovery(campaignId, input);
        if (!current()) return false;
        const confirmed = result.status;
        if (
          confirmed.campaign_id !== campaignId ||
          confirmed.session_id !== input.session_id ||
          confirmed.character_id !== input.character_id ||
          confirmed.revision <= input.expected_revision ||
          result.receipt.receipt_revision !== confirmed.revision
        )
          throw new ChatError('invalid_response', 'recovery_response_invalid');
        if (!applyRestOutcome(confirmed)) {
          pending.current = null;
          clearSoloRecoveryOutbox(campaignId);
          setRetryRequired(false);
          setError('recovery_result_obsolete');
          await load();
          return false;
        }
        clearSoloRecoveryOutbox(campaignId);
        pending.current = null;
        setRetryRequired(false);
        setStatus(confirmed);
        setReceipt(result.receipt);
        return true;
      } catch (cause) {
        if (!current()) return false;
        const failure = ChatError.from(cause);
        setError(failure.message);
        if (failure.code === 'http_error') {
          pending.current = null;
          clearSoloRecoveryOutbox(campaignId);
          setRetryRequired(false);
          await load();
        }
        return false;
      } finally {
        running.current = false;
        if (current()) {
          setBusy(false);
          if (!pending.current) useStore.getState().session.endMutation(input.causation_id);
        } else useStore.getState().session.endMutation(input.causation_id);
      }
    },
    [campaignId, current, load],
  );
  const perform = useCallback(
    async (intent: SoloRecoveryIntent) => {
      if (!status || pending.current || !current()) return false;
      const offered = status.recovery.actions.some(
        (action) => JSON.stringify(action) === JSON.stringify(intent),
      );
      if (!offered || !v.safeParse(SoloRecoveryIntentSchema, intent).success) return false;
      return send({
        session_id: status.session_id,
        character_id: status.character_id,
        expected_revision: status.revision,
        causation_id: crypto.randomUUID(),
        intent,
      });
    },
    [status, current, send],
  );
  const retry = useCallback(
    () => (pending.current ? send(pending.current) : Promise.resolve(false)),
    [send],
  );
  return { status, receipt, loading, busy, error, retryRequired, perform, retry, reload: load };
}
