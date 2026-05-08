/**
 * Valibot schemas for chat SSE event payloads + HTTP error bodies.
 *
 * Replaces the unsafe `as` casts that used to live in `handleEvent`. Every
 * untrusted JSON payload is parsed once through these schemas; the rest of
 * the pipeline works against narrow types.
 */

import * as v from 'valibot';
import type { ChatErrorCode } from './errors';

const KNOWN_CODES = [
  'auth_failed',
  'rate_limit',
  'network',
  'provider_error',
  'no_body',
  'http_error',
  'aborted',
  'invalid_response',
  'unknown',
] as const satisfies readonly ChatErrorCode[];

export const TextDeltaSchema = v.object({
  text: v.string(),
});

export const DoneSchema = v.object({
  reason: v.optional(v.string()),
});

export const StreamErrorSchema = v.object({
  code: v.optional(v.picklist(KNOWN_CODES)),
  message: v.optional(v.string()),
});

export const HttpErrorEnvelopeSchema = v.object({
  error: v.optional(
    v.object({
      code: v.optional(v.string()),
      message: v.optional(v.string()),
    }),
  ),
});

export type TextDeltaPayload = v.InferOutput<typeof TextDeltaSchema>;
export type DonePayload = v.InferOutput<typeof DoneSchema>;
export type StreamErrorPayload = v.InferOutput<typeof StreamErrorSchema>;
export type HttpErrorEnvelope = v.InferOutput<typeof HttpErrorEnvelopeSchema>;

export function safeParseText(data: unknown): TextDeltaPayload | null {
  const result = v.safeParse(TextDeltaSchema, data);
  return result.success ? result.output : null;
}

export function safeParseDone(data: unknown): DonePayload | null {
  const result = v.safeParse(DoneSchema, data);
  return result.success ? result.output : null;
}

export function safeParseStreamError(data: unknown): StreamErrorPayload | null {
  const result = v.safeParse(StreamErrorSchema, data);
  return result.success ? result.output : null;
}

export function safeParseHttpError(data: unknown): HttpErrorEnvelope | null {
  const result = v.safeParse(HttpErrorEnvelopeSchema, data);
  return result.success ? result.output : null;
}

export const ToolCallStartSchema = v.object({
  id: v.string(),
  tool_name: v.string(),
  round: v.number(),
});

export const ToolCallResultSchema = v.object({
  id: v.string(),
  tool_name: v.string(),
  args: v.unknown(),
  result: v.unknown(),
  is_error: v.boolean(),
  round: v.number(),
});

export const AgentDoneSchema = v.object({
  total_rounds: v.number(),
});

export type ToolCallStartPayload = v.InferOutput<typeof ToolCallStartSchema>;
export type ToolCallResultPayload = v.InferOutput<typeof ToolCallResultSchema>;
export type AgentDonePayload = v.InferOutput<typeof AgentDoneSchema>;

export function safeParseToolCallStart(data: unknown): ToolCallStartPayload | null {
  const r = v.safeParse(ToolCallStartSchema, data);
  return r.success ? r.output : null;
}

export function safeParseToolCallResult(data: unknown): ToolCallResultPayload | null {
  const r = v.safeParse(ToolCallResultSchema, data);
  return r.success ? r.output : null;
}

export function safeParseAgentDone(data: unknown): AgentDonePayload | null {
  const r = v.safeParse(AgentDoneSchema, data);
  return r.success ? r.output : null;
}
