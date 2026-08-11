import { useCallback } from 'react';
import { streamAgentTurn } from '../../api/agent';
import { ChatError } from '../../api/errors';
import { buildBoardSnapshot } from './model/boardSnapshot';
import type { AgentTurnPorts } from './model/ports';
import { createAgentEventReducerState, reduceAgentEvent } from './model/reduceAgentEvent';

export function useAgentTurn(ports: AgentTurnPorts) {
  const send = useCallback(
    async (text: string, images?: Parameters<AgentTurnPorts['chat']['appendUser']>[1]) => {
      if (!text.trim() || ports.chat.isStreaming()) return;
      const { campaignId, sessionId } = ports.session.ensure();
      ports.chat.clearTurnEvents();
      const history = ports.chat.history();
      ports.chat.appendUser(text, images);
      const controller = new AbortController();
      ports.chat.begin(controller);
      const board = buildBoardSnapshot(ports.combat.boardState(), ports.session.currentSceneName());
      const reducerState = createAgentEventReducerState();
      try {
        await streamAgentTurn({
          campaignId,
          sessionId,
          playerMessage: text,
          history,
          ...(images && images.length > 0 ? { images } : {}),
          ...(board ? { board } : {}),
          signal: controller.signal,
          onEvent: (event) => reduceAgentEvent(event, { campaignId }, reducerState, ports),
        });
      } catch (error) {
        ports.chat.setError(ChatError.from(error).toPayload());
      } finally {
        ports.chat.finalize();
        ports.chat.end();
      }
    },
    [ports],
  );
  return { send, cancel: ports.chat.abort };
}
