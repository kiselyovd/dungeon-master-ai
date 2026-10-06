import { useCallback, useRef } from 'react';
import { streamAgentTurn } from '../../api/agent';
import type { AgentTurnRequest } from '../../api/contracts/agent';
import { ChatError } from '../../api/errors';
import { buildBoardSnapshot } from './model/boardSnapshot';
import type { AgentTurnPorts } from './model/ports';
import { createAgentEventReducerState, reduceAgentEvent } from './model/reduceAgentEvent';

export function useAgentTurn(ports: AgentTurnPorts) {
  const pending = useRef<{ signature: string; input: AgentTurnRequest } | null>(null);
  const send = useCallback(
    async (text: string, images?: Parameters<AgentTurnPorts['chat']['appendUser']>[1]) => {
      if (!text.trim() || ports.chat.isStreaming() || ports.session.mutationPending?.()) return;
      const active = ports.session.current();
      if (active === null) {
        ports.chat.setError({ code: 'campaign_required', message: 'campaign_required' });
        return;
      }
      const { campaignId, sessionId } = active;
      const generation = ports.session.generation?.();
      const signature = JSON.stringify({ campaignId, sessionId, generation, text, images });
      const replay = pending.current?.signature === signature ? pending.current.input : null;
      const isCurrent = () => {
        const current = ports.session.current();
        return (
          current?.campaignId === campaignId &&
          current.sessionId === sessionId &&
          (generation === undefined || ports.session.generation?.() === generation)
        );
      };
      ports.chat.clearTurnEvents();
      if (replay) ports.chat.prepareReplay?.(replay.history.length);
      const history = structuredClone(ports.chat.history());
      ports.chat.appendUser(text, images);
      const controller = new AbortController();
      ports.chat.begin(controller);
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
      pending.current = { signature, input };
      const reducerState = createAgentEventReducerState();
      try {
        await streamAgentTurn({
          ...input,
          ...(replay ? { resume: true } : {}),
          signal: controller.signal,
          onEvent: (event) => {
            if (isCurrent() && !controller.signal.aborted)
              reduceAgentEvent(event, { campaignId }, reducerState, ports);
          },
        });
        if (isCurrent() && pending.current?.input.requestId === input.requestId)
          pending.current = null;
      } catch (error) {
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
  return { send, cancel: ports.chat.abort };
}
