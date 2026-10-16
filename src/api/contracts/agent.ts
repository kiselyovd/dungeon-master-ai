import type { ChatErrorCode } from '../errors';
import {
  safeParseAgentDone,
  safeParseDone,
  safeParseImageGenerated,
  safeParseReasoningText,
  safeParseStreamError,
  safeParseText,
  safeParseToolCallResult,
  safeParseToolCallStart,
  safeParseVideoGenerated,
} from '../schemas';

export type AgentMessagePart =
  | { type: 'text'; text: string }
  | { type: 'image'; mime: string; data_b64: string; name?: string | null };

export interface AgentHistoryMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  parts?: AgentMessagePart[];
}

export type AgentWireMessage =
  | { role: 'user'; parts: AgentMessagePart[] }
  | { role: 'assistant' | 'system'; content: string };

export interface AgentTurnRequest {
  campaignId: string;
  sessionId: string;
  playerMessage: string;
  history: AgentHistoryMessage[];
  images?: AgentMessagePart[];
  board?: string;
  model?: string;
  signal?: AbortSignal;
}

export type ImageSource = { type: 'generated' } | { type: 'bundled'; assetId: string };

export type AgentEvent =
  | { type: 'text_delta'; text: string }
  | { type: 'reasoning_text'; text: string }
  | { type: 'tool_call_start'; id: string; toolName: string; round: number }
  | {
      type: 'tool_call_result';
      id: string;
      toolName: string;
      args: unknown;
      result: unknown;
      isError: boolean;
      round: number;
      handledBy: string;
    }
  | {
      type: 'image_generated';
      dataUrl: string;
      toolCallId?: string;
      kind: 'map' | 'chat';
      source: ImageSource;
    }
  | { type: 'video_generated'; dataUrl: string; toolCallId?: string; kind: 'chat' }
  | { type: 'agent_done'; totalRounds: number }
  | { type: 'done' }
  | { type: 'error'; code: ChatErrorCode; message: string };

export interface RawAgentEvent {
  event: string;
  data: unknown;
}

export function toAgentWireMessage(message: AgentHistoryMessage): AgentWireMessage {
  if (message.role === 'user') {
    const parts =
      message.parts && message.parts.length > 0
        ? message.parts
        : [{ type: 'text' as const, text: message.content }];
    return { role: 'user', parts };
  }
  return { role: message.role, content: message.content };
}

export function decodeAgentEvent(raw: RawAgentEvent): AgentEvent | null {
  switch (raw.event) {
    case 'text_delta': {
      const payload = safeParseText(raw.data);
      return payload ? { type: 'text_delta', text: payload.text } : null;
    }
    case 'reasoning_text': {
      const payload = safeParseReasoningText(raw.data);
      return payload ? { type: 'reasoning_text', text: payload.text } : null;
    }
    case 'tool_call_start': {
      const payload = safeParseToolCallStart(raw.data);
      return payload
        ? {
            type: 'tool_call_start',
            id: payload.id,
            toolName: payload.tool_name,
            round: payload.round,
          }
        : null;
    }
    case 'tool_call_result': {
      const payload = safeParseToolCallResult(raw.data);
      return payload
        ? {
            type: 'tool_call_result',
            id: payload.id,
            toolName: payload.tool_name,
            args: payload.args,
            result: payload.result,
            isError: payload.is_error,
            round: payload.round,
            handledBy: payload.handled_by,
          }
        : null;
    }
    case 'image_generated': {
      const payload = safeParseImageGenerated(raw.data);
      if (!payload) return null;
      return {
        type: 'image_generated',
        dataUrl: `data:${payload.mime_type};base64,${payload.image_b64}`,
        ...(payload.tool_call_id ? { toolCallId: payload.tool_call_id } : {}),
        kind: payload.kind === 'map' ? 'map' : 'chat',
        source:
          payload.source === 'bundled' && payload.asset_id
            ? { type: 'bundled', assetId: payload.asset_id }
            : { type: 'generated' },
      };
    }
    case 'video_generated': {
      const payload = safeParseVideoGenerated(raw.data);
      if (!payload) return null;
      return {
        type: 'video_generated',
        dataUrl: `data:${payload.mime_type};base64,${payload.video_b64}`,
        ...(payload.tool_call_id ? { toolCallId: payload.tool_call_id } : {}),
        kind: 'chat',
      };
    }
    case 'agent_done': {
      const payload = safeParseAgentDone(raw.data);
      return payload ? { type: 'agent_done', totalRounds: payload.total_rounds } : null;
    }
    case 'done':
      return safeParseDone(raw.data) ? { type: 'done' } : null;
    case 'error': {
      const payload = safeParseStreamError(raw.data);
      return {
        type: 'error',
        code: payload?.code ?? 'provider_error',
        message: payload?.message ?? 'agent error',
      };
    }
    default:
      return null;
  }
}
