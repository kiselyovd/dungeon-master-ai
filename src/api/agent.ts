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

function transportEventDebugEnabled(): boolean {
  if (!import.meta.env.DEV || typeof localStorage === 'undefined') return false;
  try {
    return localStorage.getItem('dmai.debug.agentTransport') === '1';
  } catch {
    return false;
  }
}

/** Stream one validated agent turn without depending on UI state or components. */
export async function streamAgentTurn(opts: AgentTurnOptions): Promise<void> {
  const requestId = `agent-${nextRequestSequence++}`;
  const startedAt = performance.now();
  const url = await backendUrl('/agent/turn');
  const body = JSON.stringify({
    request_id: opts.requestId ?? crypto.randomUUID(),
    resume: opts.resume ?? false,
    ...(opts.clarification ? { clarification: opts.clarification } : {}),
    campaign_id: opts.campaignId,
    session_id: opts.sessionId,
    player_message: opts.playerMessage,
    history: opts.wireHistory ?? opts.history.map(toAgentWireMessage),
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
    if (response.status === 409) {
      const failure: unknown = await response.json().catch(() => null);
      if (failure && typeof failure === 'object' && 'error' in failure) {
        const detail = failure.error;
        if (
          detail &&
          typeof detail === 'object' &&
          'code' in detail &&
          (detail.code === 'agent_original_environment_changed' ||
            detail.code === 'agent_original_environment_unavailable')
        ) {
          throw new ChatError(detail.code, detail.code);
        }
      }
    }
    throw new ChatError('http_error', `HTTP ${response.status}`);
  }

  const reader = response.body.getReader();
  const decoder = new SseStreamDecoder();
  let byteCount = 0;
  let eventSequence = 0;
  let terminal = false;
  const logIndividualEvents = transportEventDebugEnabled();

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
      if (logIndividualEvents) {
        console.debug('[agent.transport]', {
          requestId,
          eventKind: event.type,
          sequence: eventSequence,
          byteCount,
        });
      }
      if (event.type === 'error') throw new ChatError(event.code, event.message);
      if (
        event.type === 'agent_done' ||
        event.type === 'done' ||
        event.type === 'agent_needs_input'
      )
        terminal = true;
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
    if (!terminal) throw new ChatError('network', 'agent_stream_incomplete');
    console.debug('[agent.transport]', {
      requestId,
      eventKind: 'stream_complete',
      sequence: eventSequence,
      byteCount,
      durationMs: Math.round(performance.now() - startedAt),
    });
  } catch (error) {
    throw ChatError.from(error);
  } finally {
    reader.releaseLock();
  }
}

export type { AgentEvent, AgentHistoryMessage, AgentMessagePart } from './contracts/agent';
