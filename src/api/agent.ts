import type { ChatMessage, MessagePart } from '../state/chat';
import { backendUrl } from './client';
import { ChatError } from './errors';
import {
  safeParseAgentDone,
  safeParseDone,
  safeParseImageGenerated,
  safeParseReasoningText,
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
  /** Image attachments staged for this turn (vision). Omitted for text-only. [F2] */
  images?: MessagePart[];
  /**
   * Pre-formatted snapshot of the live VTT board (scene, round, initiative
   * order, each token's HP/AC/grid position/conditions). Injected into the
   * agent's system context so the DM narrates from the real board - including
   * positions after the player drags a token. Omitted outside combat.
   */
  board?: string;
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
    handledBy: string,
  ) => void;
  onAgentDone: (totalRounds: number) => void;
  onReasoningDelta?: (text: string) => void;
  /**
   * A scene/map image the agent produced via `generate_image`. Delivered as a
   * ready-to-use data URL (`data:<mime>;base64,<...>`) so the caller can drop
   * it straight into an `<img src>` or store it for the VTT background. [M11]
   */
  onImageGenerated?: (dataUrl: string, toolCallId?: string) => void;
}

/**
 * Convert a chat-slice `ChatMessage` into the backend wire shape for the
 * `/agent/turn` `history` field. The backend `ChatMessage` enum tags on `role`
 * and requires `ChatMessage::User { parts: [...] }` (text + optional images);
 * assistant/system carry `content`. Sending the raw slice message (which only
 * has `content`, plus frontend-only `id`/`sequenceIndex`) makes the backend
 * 422 with "history[i]: missing field `parts`" on every turn that has history.
 */
function toAgentWireMessage(m: ChatMessage): Record<string, unknown> {
  if (m.role === 'user') {
    const parts = m.parts && m.parts.length > 0 ? m.parts : [{ type: 'text', text: m.content }];
    return { role: 'user', parts };
  }
  return { role: m.role, content: m.content };
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
    history: opts.history.map(toAgentWireMessage),
    model: opts.model,
    images: opts.images ?? [],
    ...(opts.board ? { board: opts.board } : {}),
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
      if (p)
        opts.onToolCallResult(
          p.id,
          p.tool_name,
          p.args,
          p.result,
          p.is_error,
          p.round,
          p.handled_by,
        );
      break;
    }
    case 'reasoning_text': {
      const p = safeParseReasoningText(data);
      if (p && opts.onReasoningDelta) opts.onReasoningDelta(p.text);
      break;
    }
    case 'image_generated': {
      const p = safeParseImageGenerated(data);
      if (p && opts.onImageGenerated) {
        opts.onImageGenerated(`data:${p.mime_type};base64,${p.image_b64}`, p.tool_call_id);
      }
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
