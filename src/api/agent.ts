import { backendUrl } from './client';
import {
  type AgentEvent,
  type AgentTurnRequest,
  decodeAgentEvent,
  toAgentWireMessage,
} from './contracts/agent';
import { ChatError } from './errors';
import { SseStreamDecoder } from './sseStream';

export interface AgentTurnOptions extends AgentTurnRequest {
  onEvent: (event: AgentEvent) => void;
}

let nextRequestSequence = 1;

/** Stream one validated agent turn without depending on UI state or components. */
export async function streamAgentTurn(opts: AgentTurnOptions): Promise<void> {
  const requestId = `agent-${nextRequestSequence++}`;
  const startedAt = performance.now();
  const url = await backendUrl('/agent/turn');
  const body = JSON.stringify({
    campaign_id: opts.campaignId,
    session_id: opts.sessionId,
    player_message: opts.playerMessage,
    history: opts.history.map(toAgentWireMessage),
    model: opts.model,
    images: opts.images ?? [],
    ...(opts.board ? { board: opts.board } : {}),
  });
  const init: RequestInit = {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'text/event-stream' },
    body,
    ...(opts.signal ? { signal: opts.signal } : {}),
  };

  let response: Response;
  try {
    response = await fetch(url, init);
  } catch (error) {
    throw ChatError.from(error);
  }
  if (!response.ok || !response.body) {
    throw new ChatError('http_error', `HTTP ${response.status}`);
  }

  const reader = response.body.getReader();
  const decoder = new SseStreamDecoder();
  let byteCount = 0;
  let eventSequence = 0;

  const dispatch = (rawEvents: ReturnType<SseStreamDecoder['finish']>): void => {
    for (const raw of rawEvents) {
      const event = decodeAgentEvent(raw);
      if (!event) {
        console.warn('[agent.transport]', {
          requestId,
          code: 'invalid_agent_event',
          eventKind: raw.event,
          sequence: eventSequence,
        });
        continue;
      }
      eventSequence += 1;
      console.debug('[agent.transport]', {
        requestId,
        eventKind: event.type,
        sequence: eventSequence,
        byteCount,
      });
      if (event.type === 'error') throw new ChatError(event.code, event.message);
      opts.onEvent(event);
    }
  };

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      byteCount += value.byteLength;
      dispatch(decoder.push(value));
    }
    dispatch(decoder.finish());
    console.debug('[agent.transport]', {
      requestId,
      eventKind: 'stream_complete',
      sequence: eventSequence,
      byteCount,
      durationMs: Math.round(performance.now() - startedAt),
    });
  } catch (error) {
    throw ChatError.from(error);
  }
}

export type { AgentEvent, AgentHistoryMessage, AgentMessagePart } from './contracts/agent';
