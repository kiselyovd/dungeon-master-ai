import { useMemo } from 'react';
import * as v from 'valibot';
import { RestStatusSchema } from '../api/contracts/rest';
import type { AgentTurnPorts } from '../features/agent-turn/model/ports';
import { useAgentTurn as useFeatureAgentTurn } from '../features/agent-turn/useAgentTurn';
import { projectionFromToolResult } from '../features/combat/model/combatProjection';
import { applyAdventureToolOutcome } from '../state/adventureOutcome';
import { applyCombatOutcome } from '../state/combatOutcome';
import type { Disposition } from '../state/npc';
import { applyRestOutcome } from '../state/restOutcome';
import { useStore } from '../state/useStore';

/** Application composition adapter; feature logic only receives narrow ports. */
export function useAgentTurn() {
  const ports = useMemo<AgentTurnPorts>(
    () => ({
      chat: {
        prepareReplay: (historyLength) =>
          useStore.setState((state) => ({
            chat: {
              ...state.chat,
              messages: state.chat.messages.slice(0, historyLength),
              _nextSeq: historyLength,
              streamingAssistant: null,
              streamingReasoning: null,
              chatStreamEvents: [],
            },
          })),
        isStreaming: () => useStore.getState().chat.isStreaming,
        history: () => useStore.getState().chat.messages,
        appendUser: (text, images) => useStore.getState().chat.appendUser(text, images),
        appendText: (text) => useStore.getState().chat.appendAssistantDelta(text),
        appendReasoning: (text) => useStore.getState().chat.appendReasoningDelta(text),
        begin: (controller) => useStore.getState().chat.beginStream(controller),
        end: () => useStore.getState().chat.endStream(),
        abort: () => useStore.getState().chat.abort(),
        finalize: () => useStore.getState().chat.finalizeAssistant(),
        clearTurnEvents: () => useStore.getState().chat.clearStreamEvents(),
        setError: (error) => useStore.getState().chat.setError(error),
        addToolStart: (id, toolName, args, round) =>
          useStore.getState().chat.addToolCallStartEvent(id, toolName, args, round),
        settleTool: (id, result, isError) =>
          useStore.getState().chat.settleToolCallEvent(id, result, isError),
        attachImage: (id, dataUrl, kind, source) =>
          useStore.getState().chat.attachStreamEventImage(id, dataUrl, kind, source),
        attachVideo: (id, dataUrl) => useStore.getState().chat.attachStreamEventVideo(id, dataUrl),
      },
      toolLog: {
        addPending: (id, toolName, args, round) =>
          useStore.getState().toolLog.addPending(id, toolName, args, round),
        settle: (id, result, isError, handledBy) =>
          useStore.getState().toolLog.settle(id, result, isError, handledBy),
        attachImage: (id, dataUrl, kind, source) =>
          useStore.getState().toolLog.attachImage(id, dataUrl, kind, source),
      },
      journal: {
        append: (entry) =>
          useStore.getState().journal.appendEntry({
            id: entry.id,
            campaign_id: entry.campaignId,
            chapter: entry.chapter,
            entry_html: entry.entryHtml,
            created_at: entry.createdAt,
          }),
      },
      npcs: {
        upsert: (npc) =>
          useStore.getState().npcs.upsertNpc({
            id: npc.id,
            campaign_id: npc.campaignId,
            name: npc.name,
            role: npc.role,
            disposition: npc.disposition as Disposition,
            trust: 0,
            facts: [{ text: npc.fact, created_at: npc.timestamp }],
            updated_at: npc.timestamp,
          }),
      },
      session: {
        acceptRevision: (value) => {
          if (!value || typeof value !== 'object') return;
          const result = value as Record<string, unknown>;
          const state = useStore.getState();
          const active = state.campaigns.active;
          if (
            active &&
            result.campaign_id === active.id &&
            result.session_id === state.session.activeSessionId &&
            typeof result.revision === 'number' &&
            Number.isSafeInteger(result.revision) &&
            result.revision >= active.revision
          )
            state.campaigns.setActive({ ...active, revision: result.revision });
        },
        acceptAdventureOutcome: (value) => {
          if (!applyAdventureToolOutcome(value))
            console.warn('[agent.adventure]', { code: 'adventure_outcome_ignored' });
        },
        generation: () => useStore.getState().session.transitionGeneration,
        mutationPending: () => useStore.getState().session.mutationId !== null,
        current: () => useStore.getState().session.current(),
        currentSceneName: () => useStore.getState().session.currentScene?.name,
        setScene: (title) =>
          useStore.getState().session.setCurrentScene({ name: title, stepCounter: 0 }),
        setMapImage: (dataUrl) => useStore.getState().session.setMapImage(dataUrl),
      },
      combat: {
        boardState: () => useStore.getState().combat,
        acceptToolResult: (_toolName, _args, result) => {
          const projection = projectionFromToolResult(result);
          if (projection) applyCombatOutcome(projection);
          if (typeof result === 'object' && result !== null && 'rest_status' in result) {
            const parsed = v.safeParse(RestStatusSchema, result.rest_status);
            if (parsed.success) applyRestOutcome(parsed.output);
          }
        },
      },
      now: () => new Date().toISOString(),
    }),
    [],
  );
  return useFeatureAgentTurn(ports);
}
