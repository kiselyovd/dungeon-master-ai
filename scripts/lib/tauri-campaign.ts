export const REQUIRED_CAMPAIGN_CHECKPOINTS = [
  'tauri_present',
  'backend_ready',
  'character_created',
  'assistant_reply_completed',
  'bundled_illustration',
  'bundled_map',
  'vtt_visible',
  'combat_started',
  'move_requested',
  'combat_revision_advanced',
  'combat_acted',
  'combat_turn_advanced',
  'combat_ended',
  'npc_observed',
  'journal_observed',
  'save_created',
  'save_restored',
] as const;

export type CampaignCheckpoint = (typeof REQUIRED_CAMPAIGN_CHECKPOINTS)[number];

type RevisionCheckpoint =
  | 'combat_started'
  | 'move_requested'
  | 'combat_revision_advanced'
  | 'combat_acted'
  | 'combat_turn_advanced'
  | 'combat_ended';

export type CampaignEvidence =
  | { checkpoint: 'tauri_present' }
  | { checkpoint: 'backend_ready'; port: number }
  | { checkpoint: 'character_created'; characterId: string }
  | { checkpoint: 'assistant_reply_completed' }
  | {
      checkpoint: 'bundled_illustration' | 'bundled_map';
      source: 'generated' | 'bundled';
      assetId: string;
    }
  | { checkpoint: 'vtt_visible' }
  | { checkpoint: RevisionCheckpoint; revision: number }
  | { checkpoint: 'npc_observed'; npcId: string }
  | { checkpoint: 'journal_observed'; journalId: string }
  | { checkpoint: 'save_created' | 'save_restored'; saveId: string };

export type CampaignEvidenceResult =
  | { ok: true; completed: typeof REQUIRED_CAMPAIGN_CHECKPOINTS }
  | {
      ok: false;
      code: 'missing_evidence';
      expected: CampaignCheckpoint;
      completed: readonly CampaignCheckpoint[];
    }
  | {
      ok: false;
      code: 'out_of_order';
      expected: CampaignCheckpoint;
      received: CampaignCheckpoint;
      completed: readonly CampaignCheckpoint[];
    }
  | {
      ok: false;
      code: 'invalid_evidence';
      checkpoint: CampaignCheckpoint;
      completed: readonly CampaignCheckpoint[];
    };

export interface SafeCampaignEvidenceFile {
  schemaVersion: 1;
  status: 'passed' | 'failed';
  startedAt: string;
  finishedAt: string;
  durationMs: number;
  modelId: string | null;
  generatedImage: {
    status: 'passed' | 'failed' | 'not_runtime_tested';
    source?: 'generated';
    errorCode?: string;
  };
  checkpoints: CampaignEvidence[];
  result: CampaignEvidenceResult;
  screenshots: string[];
}

export type CapturedAgentEvent =
  | { event: 'agent_done' }
  | {
      event: 'image_generated';
      kind: 'map' | 'chat';
      source: 'generated' | 'bundled';
      assetId?: string;
    }
  | {
      event: 'tool_call_result';
      toolName: string;
      isError: boolean;
      result: unknown;
    };

export function parseCapturedAgentEvents(sse: string): CapturedAgentEvent[] {
  const captured: CapturedAgentEvent[] = [];
  for (const block of sse.split(/\r\n\r\n|\n\n|\r\r/)) {
    const lines = block.split(/\r\n|\n|\r/);
    const eventName = lines.find((line) => line.startsWith('event:'))?.slice(6).trim();
    const dataText = lines
      .filter((line) => line.startsWith('data:'))
      .map((line) => line.slice(5).trimStart())
      .join('\n');
    if (!eventName || !dataText) continue;
    let data: unknown;
    try {
      data = JSON.parse(dataText);
    } catch {
      continue;
    }
    if (!data || typeof data !== 'object' || Array.isArray(data)) continue;
    const value = data as Record<string, unknown>;
    if (eventName === 'agent_done') {
      captured.push({ event: 'agent_done' });
    } else if (
      eventName === 'image_generated' &&
      (value.kind === 'map' || value.kind === 'chat') &&
      (value.source === 'generated' || value.source === 'bundled')
    ) {
      captured.push({
        event: 'image_generated',
        kind: value.kind,
        source: value.source,
        ...(typeof value.asset_id === 'string' ? { assetId: value.asset_id } : {}),
      });
    } else if (
      eventName === 'tool_call_result' &&
      typeof value.tool_name === 'string' &&
      typeof value.is_error === 'boolean'
    ) {
      captured.push({
        event: 'tool_call_result',
        toolName: value.tool_name,
        isError: value.is_error,
        result: value.result,
      });
    }
  }
  return captured;
}

function hasSafeId(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0 && value.length <= 160;
}

function invalid(
  checkpoint: CampaignCheckpoint,
  completed: readonly CampaignCheckpoint[],
): CampaignEvidenceResult {
  return { ok: false, code: 'invalid_evidence', checkpoint, completed };
}

export function evaluateCampaignEvidence(
  evidence: readonly CampaignEvidence[],
): CampaignEvidenceResult {
  const completed: CampaignCheckpoint[] = [];
  let moveRevision: number | undefined;
  let latestCombatRevision: number | undefined;
  let illustrationAssetId: string | undefined;
  let createdSaveId: string | undefined;

  for (const item of evidence) {
    const expected = REQUIRED_CAMPAIGN_CHECKPOINTS[completed.length];
    if (item.checkpoint !== expected) {
      return {
        ok: false,
        code: 'out_of_order',
        expected: expected ?? REQUIRED_CAMPAIGN_CHECKPOINTS.at(-1)!,
        received: item.checkpoint,
        completed,
      };
    }

    switch (item.checkpoint) {
      case 'backend_ready':
        if (!Number.isInteger(item.port) || item.port <= 0 || item.port > 65_535) {
          return invalid(item.checkpoint, completed);
        }
        break;
      case 'character_created':
        if (!hasSafeId(item.characterId)) return invalid(item.checkpoint, completed);
        break;
      case 'bundled_illustration':
        if (item.source !== 'bundled' || !hasSafeId(item.assetId)) {
          return invalid(item.checkpoint, completed);
        }
        illustrationAssetId = item.assetId;
        break;
      case 'bundled_map':
        if (
          item.source !== 'bundled' ||
          !hasSafeId(item.assetId) ||
          item.assetId === illustrationAssetId
        ) {
          return invalid(item.checkpoint, completed);
        }
        break;
      case 'combat_started':
      case 'combat_acted':
      case 'combat_turn_advanced':
      case 'combat_ended':
        if (
          !Number.isInteger(item.revision) ||
          item.revision < 0 ||
          (latestCombatRevision !== undefined && item.revision <= latestCombatRevision)
        ) {
          return invalid(item.checkpoint, completed);
        }
        latestCombatRevision = item.revision;
        break;
      case 'move_requested':
        if (!Number.isInteger(item.revision) || item.revision !== latestCombatRevision) {
          return invalid(item.checkpoint, completed);
        }
        moveRevision = item.revision;
        break;
      case 'combat_revision_advanced':
        if (
          !Number.isInteger(item.revision) ||
          moveRevision === undefined ||
          item.revision <= moveRevision
        ) {
          return invalid(item.checkpoint, completed);
        }
        latestCombatRevision = item.revision;
        break;
      case 'npc_observed':
        if (!hasSafeId(item.npcId)) return invalid(item.checkpoint, completed);
        break;
      case 'journal_observed':
        if (!hasSafeId(item.journalId)) return invalid(item.checkpoint, completed);
        break;
      case 'save_created':
        if (!hasSafeId(item.saveId)) return invalid(item.checkpoint, completed);
        createdSaveId = item.saveId;
        break;
      case 'save_restored':
        if (!hasSafeId(item.saveId) || item.saveId !== createdSaveId) {
          return invalid(item.checkpoint, completed);
        }
        break;
    }

    completed.push(item.checkpoint);
  }

  if (completed.length !== REQUIRED_CAMPAIGN_CHECKPOINTS.length) {
    return {
      ok: false,
      code: 'missing_evidence',
      expected: REQUIRED_CAMPAIGN_CHECKPOINTS[completed.length]!,
      completed,
    };
  }

  return { ok: true, completed: REQUIRED_CAMPAIGN_CHECKPOINTS };
}
