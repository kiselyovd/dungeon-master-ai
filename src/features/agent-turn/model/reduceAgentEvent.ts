import type { AgentEvent, ImageSource } from '../../../api/contracts/agent';
import type { AgentTurnPorts } from './ports';

const DISPOSITIONS = new Set(['friendly', 'neutral', 'hostile', 'unknown']);

type PendingMedia =
  | {
      kind: 'image';
      dataUrl: string;
      imageKind: 'map' | 'chat';
      source: ImageSource;
      mediaRef?: import('../../../api/contracts/mediaAsset').MediaReference;
    }
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
  if (media.kind === 'image') {
    if (media.mediaRef) {
      ports.chat.attachMedia?.(id, media.mediaRef, media.imageKind, media.source);
      ports.toolLog.attachMedia?.(id, media.mediaRef, media.imageKind, media.source);
    } else {
      ports.chat.attachImage(id, media.dataUrl, media.imageKind, media.source);
      ports.toolLog.attachImage(id, media.dataUrl, media.imageKind, media.source);
    }
  } else ports.chat.attachVideo(id, media.dataUrl);
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
  if (event.toolName === 'adventure_action') ports.session.acceptAdventureOutcome?.(event.result);
  if (event.toolName === 'quick_save') ports.session.acceptRevision?.(event.result);

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
        {
          kind: 'image',
          dataUrl: event.dataUrl,
          imageKind: event.kind,
          source: event.source,
          ...(event.mediaRef ? { mediaRef: event.mediaRef } : {}),
        },
        state,
        ports,
      );
      if (event.kind === 'map') {
        if (event.mediaRef) ports.session.setMapMedia?.(event.mediaRef);
        else ports.session.setMapImage(event.dataUrl);
      }
      break;
    case 'video_generated':
      recordMedia(event.toolCallId, { kind: 'video', dataUrl: event.dataUrl }, state, ports);
      break;
    case 'agent_done':
    case 'agent_needs_input':
    case 'done':
    case 'error':
      break;
  }
}
