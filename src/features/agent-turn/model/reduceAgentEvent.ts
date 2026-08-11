import type { AgentEvent } from '../../../api/contracts/agent';
import type { AgentTurnPorts } from './ports';

const DISPOSITIONS = new Set(['friendly', 'neutral', 'hostile', 'unknown']);

type PendingMedia =
  | { kind: 'image'; dataUrl: string; imageKind: 'map' | 'chat' }
  | { kind: 'video'; dataUrl: string };

export interface AgentEventReducerState {
  started: Set<string>;
  settled: Set<string>;
  pendingMedia: Map<string, PendingMedia[]>;
}

export function createAgentEventReducerState(): AgentEventReducerState {
  return { started: new Set(), settled: new Set(), pendingMedia: new Map() };
}

export interface AgentEventContext {
  campaignId: string;
}

function recordMedia(
  id: string | undefined,
  media: PendingMedia,
  state: AgentEventReducerState,
  ports: AgentTurnPorts,
): void {
  if (!id) return;
  if (!state.started.has(id)) {
    const pending = state.pendingMedia.get(id) ?? [];
    pending.push(media);
    state.pendingMedia.set(id, pending);
    return;
  }
  if (media.kind === 'image') ports.chat.attachImage(id, media.dataUrl, media.imageKind);
  else ports.chat.attachVideo(id, media.dataUrl);
}

function flushMedia(id: string, state: AgentEventReducerState, ports: AgentTurnPorts): void {
  for (const media of state.pendingMedia.get(id) ?? []) recordMedia(id, media, state, ports);
  state.pendingMedia.delete(id);
}

function reduceToolResult(
  event: Extract<AgentEvent, { type: 'tool_call_result' }>,
  context: AgentEventContext,
  state: AgentEventReducerState,
  ports: AgentTurnPorts,
): void {
  if (state.settled.has(event.id)) return;
  state.settled.add(event.id);
  ports.toolLog.settle(event.id, event.result, event.isError, event.handledBy);
  ports.chat.settleTool(event.id, event.result, event.isError);
  if (event.isError) return;

  const args =
    event.args && typeof event.args === 'object' ? (event.args as Record<string, unknown>) : {};
  const result =
    event.result && typeof event.result === 'object'
      ? (event.result as Record<string, unknown>)
      : {};
  const timestamp = ports.now();

  if (event.toolName === 'journal_append' && result.entry_id) {
    ports.journal.append({
      id: String(result.entry_id),
      campaignId: context.campaignId,
      chapter: typeof args.chapter === 'string' ? args.chapter : null,
      entryHtml: typeof args.entry_html === 'string' ? args.entry_html : '',
      createdAt: timestamp,
    });
  }
  if (event.toolName === 'remember_npc') {
    const name = typeof args.name === 'string' ? args.name : '';
    if (name) {
      const disposition =
        typeof args.disposition === 'string' && DISPOSITIONS.has(args.disposition)
          ? args.disposition
          : 'unknown';
      ports.npcs.upsert({
        id: name,
        campaignId: context.campaignId,
        name,
        role: typeof args.role === 'string' ? args.role : '',
        disposition,
        fact: typeof args.fact === 'string' ? args.fact : '',
        timestamp,
      });
    }
  }
  if (event.toolName === 'set_scene') {
    const title = typeof args.title === 'string' ? args.title : '';
    if (title) ports.session.setScene(title);
  }

  try {
    ports.combat.acceptToolResult(event.toolName, event.args, event.result);
  } catch {
    console.warn('[agent.reducer]', {
      code: 'combat_projection_handler_failed',
      eventKind: event.type,
      entityId: event.id,
    });
  }
}

export function reduceAgentEvent(
  event: AgentEvent,
  context: AgentEventContext,
  state: AgentEventReducerState,
  ports: AgentTurnPorts,
): void {
  switch (event.type) {
    case 'text_delta':
      ports.chat.appendText(event.text);
      break;
    case 'reasoning_text':
      ports.chat.appendReasoning(event.text);
      break;
    case 'tool_call_start':
      if (state.started.has(event.id)) break;
      state.started.add(event.id);
      ports.toolLog.addPending(event.id, event.toolName, {}, event.round);
      ports.chat.addToolStart(event.id, event.toolName, {}, event.round);
      flushMedia(event.id, state, ports);
      break;
    case 'tool_call_result':
      reduceToolResult(event, context, state, ports);
      break;
    case 'image_generated':
      recordMedia(
        event.toolCallId,
        { kind: 'image', dataUrl: event.dataUrl, imageKind: event.kind },
        state,
        ports,
      );
      if (event.kind === 'map') ports.session.setMapImage(event.dataUrl);
      break;
    case 'video_generated':
      recordMedia(event.toolCallId, { kind: 'video', dataUrl: event.dataUrl }, state, ports);
      break;
    case 'agent_done':
    case 'done':
    case 'error':
      break;
  }
}
