import * as v from 'valibot';
import { backendUrl } from './client';
import type { AgentTurnRequest } from './contracts/agent';
import { ChatError } from './errors';
import { discardResponseBody } from './response';

const uuid = v.pipe(v.string(), v.uuid());
const part = v.variant('type', [
  v.strictObject({ type: v.literal('text'), text: v.string() }),
  v.strictObject({
    type: v.literal('image'),
    mime: v.string(),
    data_b64: v.string(),
    name: v.nullable(v.string()),
  }),
]);
const message = v.variant('role', [
  v.strictObject({ role: v.literal('user'), parts: v.array(part) }),
  v.strictObject({ role: v.literal('assistant'), content: v.string() }),
  v.strictObject({ role: v.literal('system'), content: v.string() }),
]);
const pending = v.nullable(
  v.strictObject({
    operation: v.strictObject({
      request_id: uuid,
      model: v.nullable(v.string()),
      request: v.strictObject({
        campaign_id: uuid,
        session_id: uuid,
        player_message: v.string(),
        history: v.array(message),
        images: v.array(part),
        board: v.nullable(v.string()),
      }),
    }),
    clarification: v.nullable(
      v.strictObject({
        id: uuid,
        command_id: v.pipe(v.number(), v.integer(), v.minValue(1)),
        question: v.string(),
        reply: v.nullable(v.string()),
      }),
    ),
  }),
);

export async function getPendingAgentClarification(
  campaignId: string,
  sessionId: string,
  signal?: AbortSignal,
): Promise<{ input: AgentTurnRequest; clarificationId?: string } | null> {
  const url = await backendUrl(
    `/agent/pending?campaign_id=${encodeURIComponent(campaignId)}&session_id=${encodeURIComponent(sessionId)}`,
  );
  const response = await fetch(url, {
    headers: { accept: 'application/json' },
    ...(signal ? { signal } : {}),
  });
  if (response.status === 404) {
    await discardResponseBody(response);
    return null;
  }
  if (!response.ok) {
    await discardResponseBody(response);
    throw new ChatError('http_error', `HTTP ${response.status}`);
  }
  const result = v.safeParse(pending, await response.json());
  if (!result.success) throw new ChatError('invalid_response', 'invalid_pending_clarification');
  if (result.output === null) return null;
  const { operation, clarification } = result.output;
  const request = operation.request;
  if (request.campaign_id !== campaignId || request.session_id !== sessionId)
    throw new ChatError('invalid_response', 'pending_clarification_scope_mismatch');
  return {
    ...(clarification && clarification.reply === null ? { clarificationId: clarification.id } : {}),
    input: {
      requestId: operation.request_id,
      campaignId,
      sessionId,
      playerMessage: request.player_message,
      wireHistory: request.history,
      history: request.history.map((entry) =>
        entry.role === 'user'
          ? {
              role: entry.role,
              content: entry.parts
                .filter((item) => item.type === 'text')
                .map((item) => item.text)
                .join('\n'),
              parts: entry.parts,
            }
          : { role: entry.role, content: entry.content },
      ),
      ...(request.images.length ? { images: request.images } : {}),
      ...(request.board !== null ? { board: request.board } : {}),
      ...(operation.model !== null ? { model: operation.model } : {}),
      ...(clarification && clarification.reply !== null
        ? { clarification: { id: clarification.id, reply: clarification.reply } }
        : {}),
    },
  };
}
