import type { ChatMessage } from '../state/chat';
import { backendUrl } from './client';
import { ChatError } from './errors';
import {
  safeParseAgentDone,
  safeParseDone,
  safeParseStreamError,
  safeParseText,
  safeParseToolCallResult,
  safeParseToolCallStart,
} from './schemas';
import { parseSseEvents } from './sse';

export interface AgentTurnOptions {
  campaignId: string;
  sessionId: string;
  playerMessage: string;
  history: ChatMessage[];
  model?: string;
  signal?: AbortSignal;

  onTextDelta: (text: string) => void;
  onToolCallStart: (id: string, toolName: string, round: number) => void;
  onToolCallResult: (
    id: string,
    toolName: string,
    args: unknown,
    result: unknown,
    isError: boolean,
    round: number,
  ) => void;
  onAgentDone: (totalRounds: number) => void;
}

/**
 * Drive the SSE agent-turn pipeline. Emits text deltas, tool-call start/result,
 * and agent_done events to the appropriate callbacks. Errors during the stream
 * surface as a thrown `ChatError` so the caller can record `lastError` once.
 */
export async function streamAgentTurn(opts: AgentTurnOptions): Promise<void> {
  const url = await backendUrl('/agent/turn');
  const body = JSON.stringify({
    campaign_id: opts.campaignId,
    session_id: opts.sessionId,
    player_message: opts.playerMessage,
    history: opts.history,
    model: opts.model,
  });

  const init: RequestInit = {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'text/event-stream' },
    body,
  };
  if (opts.signal) init.signal = opts.signal;

  let resp: Response;
  try {
    resp = await fetch(url, init);
  } catch (e) {
    throw ChatError.from(e);
  }

  if (!resp.ok || !resp.body) {
    throw new ChatError('http_error', `HTTP ${resp.status}`);
  }

  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });

      const blocks = buffer.split('\n\n');
      buffer = blocks.pop() ?? '';

      for (const block of blocks) {
        if (!block.trim()) continue;
        const events = parseSseEvents(`${block}\n\n`);
        for (const ev of events) {
          handleAgentEvent(ev.event, ev.data, opts);
        }
      }
    }
  } catch (e) {
    throw ChatError.from(e);
  }
}

function handleAgentEvent(eventName: string, data: unknown, opts: AgentTurnOptions): void {
  switch (eventName) {
    case 'text_delta': {
      const p = safeParseText(data);
      if (p) opts.onTextDelta(p.text);
      break;
    }
    case 'tool_call_start': {
      const p = safeParseToolCallStart(data);
      if (p) opts.onToolCallStart(p.id, p.tool_name, p.round);
      break;
    }
    case 'tool_call_result': {
      const p = safeParseToolCallResult(data);
      if (p) opts.onToolCallResult(p.id, p.tool_name, p.args, p.result, p.is_error, p.round);
      break;
    }
    case 'agent_done': {
      const p = safeParseAgentDone(data);
      if (p) opts.onAgentDone(p.total_rounds);
      break;
    }
    case 'done': {
      // Backwards-compat with /chat endpoint; agent stream uses agent_done.
      void safeParseDone(data);
      break;
    }
    case 'error': {
      const p = safeParseStreamError(data);
      throw new ChatError(p?.code ?? 'provider_error', p?.message ?? 'agent error');
    }
  }
}
