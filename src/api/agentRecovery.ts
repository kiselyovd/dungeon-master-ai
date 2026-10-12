import { backendUrl } from './client';
import { type AgentRecoveryProjection, parseAgentRecovery } from './contracts/agentRecovery';
import { ChatError } from './errors';
import { discardResponseBody } from './response';

export async function fetchAgentRecovery(
  campaignId: string,
  sessionId: string,
  signal?: AbortSignal,
): Promise<AgentRecoveryProjection | null> {
  const url = await backendUrl(
    `/agent/recovery?campaign_id=${encodeURIComponent(campaignId)}&session_id=${encodeURIComponent(sessionId)}`,
  );
  const response = await fetch(url, {
    headers: { accept: 'application/json' },
    ...(signal ? { signal } : {}),
  });
  if (!response.ok) {
    await discardResponseBody(response);
    throw new ChatError('http_error', `HTTP ${response.status}`);
  }
  let result: AgentRecoveryProjection | null;
  try {
    result = parseAgentRecovery(await response.json());
  } catch {
    throw new ChatError('invalid_response', 'agent_recovery_invalid');
  }
  if (
    result &&
    (result.scope.campaign_id !== campaignId ||
      result.scope.session_id !== sessionId ||
      result.scope.authority.kind !== 'solo')
  ) {
    throw new ChatError('invalid_response', 'agent_recovery_scope_mismatch');
  }
  return result;
}
