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
