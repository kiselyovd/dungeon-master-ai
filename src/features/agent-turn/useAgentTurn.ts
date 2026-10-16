import { useCallback, useRef } from 'react';
import { streamAgentTurn } from '../../api/agent';
import { getPendingAgentClarification } from '../../api/agentPending';
import type { AgentTurnRequest } from '../../api/contracts/agent';
import { ChatError } from '../../api/errors';
import { buildBoardSnapshot } from './model/boardSnapshot';
import type { AgentTurnPorts } from './model/ports';
import { createAgentEventReducerState, reduceAgentEvent } from './model/reduceAgentEvent';

function awaitPreparation(task: Promise<void> | undefined, signal: AbortSignal): Promise<void> {
  if (!task) return Promise.resolve();
  return new Promise((resolve, reject) => {
    const abort = () => reject(new DOMException('provider_preparation_aborted', 'AbortError'));
    if (signal.aborted) {
      abort();
      return;
    }
    signal.addEventListener('abort', abort, { once: true });
    task.then(
      () => {
        signal.removeEventListener('abort', abort);
        resolve();
      },
      (error) => {
        signal.removeEventListener('abort', abort);
        reject(error);
      },
    );
  });
}

export function useAgentTurn(ports: AgentTurnPorts) {
  const pending = useRef<{
    signature: string;
    scopeSignature: string;
    input: AgentTurnRequest;
    started: boolean;
    clarificationId?: string;
    replayHistoryLength: number;
  } | null>(null);
  const probedScopes = useRef(new Set<string>());
  const probing = useRef(false);
  const send = useCallback(
    async (text: string, images?: Parameters<AgentTurnPorts['chat']['appendUser']>[1]) => {
      if (
        !text.trim() ||
        probing.current ||
        ports.chat.isStreaming() ||
        ports.session.mutationPending?.()
      )
        return;
      const active = ports.session.current();
      if (active === null) {
        ports.chat.setError({ code: 'campaign_required', message: 'campaign_required' });
        return;
      }
      const { campaignId, sessionId } = active;
      const generation = ports.session.generation?.();
      const signature = JSON.stringify({ campaignId, sessionId, generation, text, images });
      const scopeSignature = JSON.stringify({ campaignId, sessionId, generation });
      const isCurrent = () => {
        const current = ports.session.current();
        return (
          current?.campaignId === campaignId &&
          current.sessionId === sessionId &&
          (generation === undefined || ports.session.generation?.() === generation)
        );
      };
      const controller = new AbortController();
      ports.chat.begin(controller);
      if (!probedScopes.current.has(scopeSignature)) {
        probing.current = true;
        let readCompleted = false;
        try {
          const restored = await getPendingAgentClarification(
            campaignId,
            sessionId,
            controller.signal,
          );
          if (!isCurrent() || controller.signal.aborted) return;
          probedScopes.current.add(scopeSignature);
          if (restored)
            pending.current = {
              signature:
                !restored.clarificationId &&
                text === (restored.input.clarification?.reply ?? restored.input.playerMessage)
                  ? signature
                  : '',
              scopeSignature,
              input: restored.input,
              started: true,
              ...(restored.clarificationId ? { clarificationId: restored.clarificationId } : {}),
              replayHistoryLength:
                !restored.clarificationId &&
                pending.current?.scopeSignature === scopeSignature &&
                pending.current.input.requestId === restored.input.requestId
                  ? pending.current.replayHistoryLength
                  : ports.chat.history().length,
            };
          readCompleted = true;
        } catch (error) {
          if (isCurrent()) ports.chat.setError(ChatError.from(error).toPayload());
          return;
        } finally {
          probing.current = false;
          if (!readCompleted && isCurrent()) {
            ports.chat.finalize();
            ports.chat.end();
          }
        }
      }
      const awaited = pending.current;
      const answering =
        awaited?.scopeSignature === scopeSignature && awaited.clarificationId
          ? { input: awaited.input, id: awaited.clarificationId }
          : null;
      const replay = answering
        ? { ...answering.input, clarification: { id: answering.id, reply: text } }
        : pending.current?.signature === signature
          ? pending.current.input
          : null;
      const resume = Boolean(replay && pending.current?.started);
      const replayHistoryLength =
        replay && !answering
          ? (pending.current?.replayHistoryLength ?? replay.history.length)
          : ports.chat.history().length;
      ports.chat.clearTurnEvents();
      if (replay) ports.chat.prepareReplay?.(replayHistoryLength);
      const history = structuredClone(ports.chat.history());
      ports.chat.appendUser(text, images);
      const board = buildBoardSnapshot(ports.combat.boardState(), ports.session.currentSceneName());
      const input: AgentTurnRequest = replay ?? {
        requestId: crypto.randomUUID(),
        campaignId,
        sessionId,
        playerMessage: text,
        history,
        ...(images && images.length > 0 ? { images: structuredClone(images) } : {}),
        ...(board ? { board } : {}),
      };
      pending.current = { signature, scopeSignature, input, started: resume, replayHistoryLength };
      const reducerState = createAgentEventReducerState();
      try {
        await awaitPreparation(ports.prepareProvider?.(), controller.signal);
        if (!isCurrent() || controller.signal.aborted) return;
        if (pending.current?.input.requestId === input.requestId) pending.current.started = true;
        await streamAgentTurn({
          ...input,
          ...(resume ? { resume: true } : {}),
          signal: controller.signal,
          onEvent: (event) => {
            const currentPending = pending.current;
            if (
              isCurrent() &&
              !controller.signal.aborted &&
              event.type === 'agent_needs_input' &&
              currentPending &&
              currentPending.input.requestId === input.requestId
            )
              currentPending.clarificationId = event.clarificationId;
            if (isCurrent() && !controller.signal.aborted)
              reduceAgentEvent(event, { campaignId }, reducerState, ports);
          },
        });
        if (
          isCurrent() &&
          pending.current?.input.requestId === input.requestId &&
          !pending.current.clarificationId
        )
          pending.current = null;
      } catch (error) {
        probedScopes.current.delete(scopeSignature);
        if (isCurrent()) ports.chat.setError(ChatError.from(error).toPayload());
      } finally {
        if (isCurrent()) {
          ports.chat.finalize();
          ports.chat.end();
        }
      }
    },
    [ports],
  );
  const prepareRecovery = useCallback(
    (input: AgentTurnRequest, clarificationId?: string) => {
      const active = ports.session.current();
      if (
        !active ||
        active.campaignId !== input.campaignId ||
        active.sessionId !== input.sessionId ||
        ports.chat.isStreaming()
      )
        return false;
      const generation = ports.session.generation?.();
      const scopeSignature = JSON.stringify({
        campaignId: input.campaignId,
        sessionId: input.sessionId,
        generation,
      });
      pending.current = {
        signature: JSON.stringify({
          campaignId: input.campaignId,
          sessionId: input.sessionId,
          generation,
          text: input.clarification?.reply ?? input.playerMessage,
          images: input.images,
        }),
        scopeSignature,
        input,
        started: true,
        replayHistoryLength: input.history.length,
        ...(clarificationId ? { clarificationId } : {}),
      };
      probedScopes.current.add(scopeSignature);
      return true;
    },
    [ports],
  );
  const resumeFrom = useCallback(
    async (input: AgentTurnRequest, clarificationId?: string, reply?: string) => {
      if (prepareRecovery(input, clarificationId))
        await send(reply ?? input.clarification?.reply ?? input.playerMessage, input.images);
    },
    [prepareRecovery, send],
  );
  return { send, cancel: ports.chat.abort, prepareRecovery, resumeFrom };
}
