/**
 * Closed string-literal union of error codes the chat pipeline can surface.
 *
 * Provider-specific codes (e.g. `local_oom` for the future embedded mistralrs
 * provider in M4) extend this list; UI mapping in `errors.json` mirrors each
 * variant 1:1 so an exhaustive `switch` in the renderer is enforced by TS.
 */
export type ChatErrorCode =
  | 'auth_failed'
  | 'rate_limit'
  | 'network'
  | 'provider_error'
  | 'no_body'
  | 'http_error'
  | 'aborted'
  | 'invalid_response'
  | 'unknown';

export interface ChatErrorPayload {
  code: ChatErrorCode;
  message: string;
}

/**
 * Thrown by every failure path in `streamChat`. Carries a stable `code` for
 * i18n + telemetry plus the original `cause` for debugging. Catch with
 * `instanceof ChatError` and read `.code` + `.message`.
 */
export class ChatError extends Error {
  readonly code: ChatErrorCode;

  constructor(code: ChatErrorCode, message: string, options?: { cause?: unknown }) {
    super(message, options);
    this.name = 'ChatError';
    this.code = code;
  }

  toPayload(): ChatErrorPayload {
    return { code: this.code, message: this.message };
  }

  /** Best-effort coercion of arbitrary throwables into a ChatError. */
  static from(err: unknown): ChatError {
    if (err instanceof ChatError) return err;
    if (err instanceof DOMException && err.name === 'AbortError') {
      return new ChatError('aborted', 'request was aborted', { cause: err });
    }
    if (err instanceof TypeError) {
      // fetch() throws TypeError on network failure, DNS, CORS, etc.
      return new ChatError('network', err.message, { cause: err });
    }
    if (err instanceof Error) {
      return new ChatError('unknown', err.message, { cause: err });
    }
    return new ChatError('unknown', String(err));
  }
}
