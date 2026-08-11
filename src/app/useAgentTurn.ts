import { useMemo } from 'react';
import type { AgentTurnPorts } from '../features/agent-turn/model/ports';
import { useAgentTurn as useFeatureAgentTurn } from '../features/agent-turn/useAgentTurn';
import { projectionFromToolResult } from '../features/combat/model/combatProjection';
import type { Disposition } from '../state/npc';
import { useStore } from '../state/useStore';

/** Application composition adapter; feature logic only receives narrow ports. */
export function useAgentTurn() {
  const ports = useMemo<AgentTurnPorts>(
    () => ({
      chat: {
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
        attachImage: (id, dataUrl, kind) =>
          useStore.getState().chat.attachStreamEventImage(id, dataUrl, kind),
        attachVideo: (id, dataUrl) => useStore.getState().chat.attachStreamEventVideo(id, dataUrl),
      },
      toolLog: {
        addPending: (id, toolName, args, round) =>
          useStore.getState().toolLog.addPending(id, toolName, args, round),
        settle: (id, result, isError, handledBy) =>
          useStore.getState().toolLog.settle(id, result, isError, handledBy),
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
        ensure: () => useStore.getState().session.ensureSession(),
        currentSceneName: () => useStore.getState().session.currentScene?.name,
        setScene: (title) =>
          useStore.getState().session.setCurrentScene({ name: title, stepCounter: 0 }),
        setMapImage: (dataUrl) => useStore.getState().session.setMapImage(dataUrl),
      },
      combat: {
        boardState: () => useStore.getState().combat,
        acceptToolResult: (_toolName, _args, result) => {
          const projection = projectionFromToolResult(result);
          if (projection) useStore.getState().combat.replaceProjection(projection);
        },
      },
      now: () => new Date().toISOString(),
    }),
    [],
  );
  return useFeatureAgentTurn(ports);
}
