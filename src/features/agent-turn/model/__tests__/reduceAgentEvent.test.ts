import { describe, expect, it, vi } from 'vitest';
import type { AgentTurnPorts } from '../ports';
import { createAgentEventReducerState, reduceAgentEvent } from '../reduceAgentEvent';

function createPorts(): AgentTurnPorts {
  return {
    chat: {
      isStreaming: vi.fn(() => false),
      history: vi.fn(() => []),
      appendUser: vi.fn(),
      appendText: vi.fn(),
      appendReasoning: vi.fn(),
      begin: vi.fn(),
      end: vi.fn(),
      abort: vi.fn(),
      finalize: vi.fn(),
      clearTurnEvents: vi.fn(),
      setError: vi.fn(),
      addToolStart: vi.fn(),
      settleTool: vi.fn(),
      attachImage: vi.fn(),
      attachVideo: vi.fn(),
    },
    toolLog: { addPending: vi.fn(), settle: vi.fn(), attachImage: vi.fn() },
    journal: { append: vi.fn() },
    npcs: { upsert: vi.fn() },
    session: {
      ensure: vi.fn(() => ({ campaignId: 'campaign-1', sessionId: 'session-1' })),
      currentSceneName: vi.fn(() => undefined),
      setScene: vi.fn(),
      setMapImage: vi.fn(),
    },
    combat: {
      boardState: vi.fn(() => ({ active: false, round: 0, initiativeOrder: [], tokens: [] })),
      acceptToolResult: vi.fn(),
    },
    now: vi.fn(() => '2026-08-11T12:00:00.000Z'),
  };
}

describe('reduceAgentEvent', () => {
  it('applies tool starts/results once while preserving start-before-settle ordering', () => {
    const ports = createPorts();
    const state = createAgentEventReducerState();
    const context = { campaignId: 'campaign-1' };
    const start = { type: 'tool_call_start' as const, id: 'tc-1', toolName: 'roll_dice', round: 1 };
    const result = {
      type: 'tool_call_result' as const,
      id: 'tc-1',
      toolName: 'roll_dice',
      args: {},
      result: { total: 7 },
      isError: false,
      round: 1,
      handledBy: 'engine',
    };
    reduceAgentEvent(start, context, state, ports);
    reduceAgentEvent(start, context, state, ports);
    reduceAgentEvent(result, context, state, ports);
    reduceAgentEvent(result, context, state, ports);
    expect(ports.chat.addToolStart).toHaveBeenCalledTimes(1);
    expect(ports.chat.settleTool).toHaveBeenCalledTimes(1);
    expect(ports.toolLog.addPending).toHaveBeenCalledTimes(1);
    expect(ports.toolLog.settle).toHaveBeenCalledTimes(1);
  });

  it('maps journal, NPC, and scene results without exposing store state', () => {
    const ports = createPorts();
    const state = createAgentEventReducerState();
    const context = { campaignId: 'campaign-1' };
    const result = (id: string, toolName: string, args: unknown, value: unknown) => ({
      type: 'tool_call_result' as const,
      id,
      toolName,
      args,
      result: value,
      isError: false,
      round: 1,
      handledBy: 'engine',
    });
    reduceAgentEvent(
      result(
        'j1',
        'journal_append',
        { chapter: 'I', entry_html: '<p>Found it</p>' },
        { entry_id: 'entry-1' },
      ),
      context,
      state,
      ports,
    );
    reduceAgentEvent(
      result(
        'n1',
        'remember_npc',
        { name: 'Mira', role: 'Scout', disposition: 'friendly', fact: 'Knows the pass' },
        {},
      ),
      context,
      state,
      ports,
    );
    reduceAgentEvent(
      result('s1', 'set_scene', { title: 'Sunless Citadel' }, { scene_id: 'scene-1' }),
      context,
      state,
      ports,
    );
    expect(ports.journal.append).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'entry-1', campaignId: 'campaign-1' }),
    );
    expect(ports.npcs.upsert).toHaveBeenCalledWith(
      expect.objectContaining({ name: 'Mira', disposition: 'friendly' }),
    );
    expect(ports.session.setScene).toHaveBeenCalledWith('Sunless Citadel');
  });

  it('buffers media arriving before its tool card and routes maps to the board', () => {
    const ports = createPorts();
    const state = createAgentEventReducerState();
    const context = { campaignId: 'campaign-1' };
    reduceAgentEvent(
      {
        type: 'image_generated',
        toolCallId: 'tc-1',
        dataUrl: 'data:image/png;base64,AA',
        kind: 'map',
        source: { type: 'generated' },
      },
      context,
      state,
      ports,
    );
    expect(ports.chat.attachImage).not.toHaveBeenCalled();
    expect(ports.session.setMapImage).toHaveBeenCalledWith('data:image/png;base64,AA');
    reduceAgentEvent(
      { type: 'tool_call_start', id: 'tc-1', toolName: 'generate_image', round: 1 },
      context,
      state,
      ports,
    );
    expect(ports.chat.attachImage).toHaveBeenCalledWith('tc-1', 'data:image/png;base64,AA', 'map', {
      type: 'generated',
    });
  });

  it('keeps bundled provenance before and after the tool start', () => {
    const bundled = { type: 'bundled' as const, assetId: 'illustration-tavern' };
    for (const mediaFirst of [true, false]) {
      const ports = createPorts();
      const state = createAgentEventReducerState();
      const context = { campaignId: 'campaign-1' };
      const start = {
        type: 'tool_call_start' as const,
        id: 'tc-bundled',
        toolName: 'generate_illustration',
        round: 1,
      };
      const media = {
        type: 'image_generated' as const,
        toolCallId: 'tc-bundled',
        dataUrl: 'data:image/webp;base64,AA',
        kind: 'chat' as const,
        source: bundled,
      };
      for (const event of mediaFirst ? [media, start] : [start, media]) {
        reduceAgentEvent(event, context, state, ports);
      }
      expect(ports.chat.attachImage).toHaveBeenCalledWith(
        'tc-bundled',
        'data:image/webp;base64,AA',
        'chat',
        bundled,
      );
      expect(ports.toolLog.attachImage).toHaveBeenCalledWith(
        'tc-bundled',
        'data:image/webp;base64,AA',
        'chat',
        bundled,
      );
    }
  });

  it('isolates a combat projection handler failure', () => {
    const ports = createPorts();
    vi.mocked(ports.combat.acceptToolResult).mockImplementation(() => {
      throw new Error('boom');
    });
    expect(() =>
      reduceAgentEvent(
        {
          type: 'tool_call_result',
          id: 'tc-1',
          toolName: 'start_combat',
          args: {},
          result: {},
          isError: false,
          round: 1,
          handledBy: 'engine',
        },
        { campaignId: 'campaign-1' },
        createAgentEventReducerState(),
        ports,
      ),
    ).not.toThrow();
  });
});
