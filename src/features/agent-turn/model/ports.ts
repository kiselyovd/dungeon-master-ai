import type { AgentHistoryMessage, AgentMessagePart } from '../../../api/contracts/agent';
import type { ChatErrorPayload } from '../../../api/errors';

export interface BoardToken {
  id: string;
  name: string;
  hp: number;
  maxHp: number;
  ac: number;
  x: number;
  y: number;
  conditions: string[];
  isActive?: boolean;
}

export interface BoardCombatState {
  active: boolean;
  round: number;
  initiativeOrder: string[];
  tokens: BoardToken[];
}

export interface ChatPort {
  isStreaming(): boolean;
  history(): AgentHistoryMessage[];
  appendUser(text: string, images?: AgentMessagePart[]): void;
  appendText(text: string): void;
  appendReasoning(text: string): void;
  begin(controller: AbortController): void;
  end(): void;
  abort(): void;
  finalize(): void;
  clearTurnEvents(): void;
  setError(error: ChatErrorPayload): void;
  addToolStart(id: string, toolName: string, args: unknown, round: number): void;
  settleTool(id: string, result: unknown, isError: boolean): void;
  attachImage(id: string, dataUrl: string, kind: 'map' | 'chat'): void;
  attachVideo(id: string, dataUrl: string): void;
}

export interface ToolLogPort {
  addPending(id: string, toolName: string, args: unknown, round: number): void;
  settle(id: string, result: unknown, isError: boolean, handledBy: string): void;
}

export interface JournalPort {
  append(entry: {
    id: string;
    campaignId: string;
    chapter: string | null;
    entryHtml: string;
    createdAt: string;
  }): void;
}

export interface NpcPort {
  upsert(npc: {
    id: string;
    campaignId: string;
    name: string;
    role: string;
    disposition: string;
    fact: string;
    timestamp: string;
  }): void;
}

export interface SessionPort {
  ensure(): { campaignId: string; sessionId: string };
  currentSceneName(): string | undefined;
  setScene(title: string): void;
  setMapImage(dataUrl: string): void;
}

export interface CombatProjectionPort {
  boardState(): BoardCombatState;
  acceptToolResult(toolName: string, args: unknown, result: unknown): void;
}

export interface AgentTurnPorts {
  chat: ChatPort;
  toolLog: ToolLogPort;
  journal: JournalPort;
  npcs: NpcPort;
  session: SessionPort;
  combat: CombatProjectionPort;
  now(): string;
}
