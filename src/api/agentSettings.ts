/**
 * HTTP client for the M3 `POST /agent-settings` endpoint that lets the
 * frontend push the model-tab knobs (system prompt, temperature, Replicate
 * API key) into the running app-server. The endpoint accepts snake_case
 * fields; only the keys that are present are applied, so optional fields are
 * sent as `undefined` (and stripped from the JSON body).
 */
import { backendUrl } from './client';
import { ChatError } from './errors';

export interface AgentSettingsRequest {
  system_prompt?: string;
  temperature?: number;
  replicate_api_key?: string;
}

export async function postAgentSettings(req: AgentSettingsRequest): Promise<void> {
  const url = await backendUrl('/agent-settings');
  let resp: Response;
  try {
    resp = await fetch(url, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(req),
    });
  } catch (e) {
    throw ChatError.from(e);
  }
  if (!resp.ok) {
    throw new ChatError('http_error', `POST /agent-settings HTTP ${resp.status}`);
  }
}
