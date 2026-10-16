import { describe, expect, it } from 'vitest';

import {
  REQUIRED_CAMPAIGN_CHECKPOINTS,
  evaluateCampaignEvidence,
  isCombatSceneVisible,
  isChatScrollContained,
  isManualHeroButtonLabel,
  parseCapturedAgentEvents,
  resolveJournalEvidenceId,
  resolveCampaignBootstrapAction,
  resolveDmResponseTimeoutMs,
  resolveRuntimeTimeoutMs,
  type CampaignEvidence,
} from './tauri-campaign';

describe('resolveRuntimeTimeoutMs', () => {
  it('outlives the backend cold-start probe budget by default', () => {
    expect(resolveRuntimeTimeoutMs(undefined)).toBe(780_000);
  });

  it('accepts an explicit positive timeout override', () => {
    expect(resolveRuntimeTimeoutMs('900000')).toBe(900_000);
  });

  it('rejects malformed and non-positive overrides', () => {
    expect(resolveRuntimeTimeoutMs('not-a-number')).toBe(780_000);
    expect(resolveRuntimeTimeoutMs('0')).toBe(780_000);
  });
});

describe('resolveDmResponseTimeoutMs', () => {
  it('allows a multi-round local-model tool turn to finish by default', () => {
    expect(resolveDmResponseTimeoutMs(undefined)).toBe(720_000);
  });

  it('accepts an explicit positive timeout override', () => {
    expect(resolveDmResponseTimeoutMs('900000')).toBe(900_000);
  });

  it('rejects malformed and non-positive overrides', () => {
    expect(resolveDmResponseTimeoutMs('not-a-number')).toBe(720_000);
    expect(resolveDmResponseTimeoutMs('-1')).toBe(720_000);
  });
});

describe('resolveJournalEvidenceId', () => {
  it('accepts a journal entry without an optional chapter heading', () => {
    expect(resolveJournalEvidenceId(1)).toBe('observed-journal-entry');
  });

  it('does not manufacture evidence for an empty journal', () => {
    expect(resolveJournalEvidenceId(0)).toBeNull();
  });
});

describe('resolveCampaignBootstrapAction', () => {
  it('opens campaign creation before looking for the character wizard', () => {
    expect(
      resolveCampaignBootstrapAction({
        wizardVisible: false,
        campaignCreateVisible: false,
        newCampaignAvailable: true,
      }),
    ).toBe('open_campaign_create');
  });

  it('prefers the stable campaign-home action over a transient wizard', () => {
    expect(
      resolveCampaignBootstrapAction({
        wizardVisible: true,
        campaignCreateVisible: false,
        newCampaignAvailable: true,
      }),
    ).toBe('open_campaign_create');
  });

  it('completes an already open campaign draft before character creation', () => {
    expect(
      resolveCampaignBootstrapAction({
        wizardVisible: false,
        campaignCreateVisible: true,
        newCampaignAvailable: false,
      }),
    ).toBe('complete_campaign_create');
  });

  it('leaves an already visible character wizard untouched', () => {
    expect(
      resolveCampaignBootstrapAction({
        wizardVisible: true,
        campaignCreateVisible: false,
        newCampaignAvailable: false,
        gameTableVisible: false,
      }),
    ).toBe('ready');
  });

  it('resumes an already prepared game table without reopening character creation', () => {
    expect(
      resolveCampaignBootstrapAction({
        wizardVisible: false,
        campaignCreateVisible: false,
        newCampaignAvailable: false,
        gameTableVisible: true,
      }),
    ).toBe('game_ready');
  });
});

describe('isChatScrollContained', () => {
  const contained = {
    viewportHeight: 900,
    documentHeight: 900,
    bodyHeight: 900,
    chatPanelHeight: 836,
    chatClientHeight: 610,
    chatScrollHeight: 2_700,
    bodyOverflow: 'hidden',
    chatOverflowY: 'auto',
  };

  it('accepts a fixed application shell with an independently scrolling transcript', () => {
    expect(isChatScrollContained(contained)).toBe(true);
  });

  it('rejects the regression where chat content expands the whole window', () => {
    expect(
      isChatScrollContained({
        ...contained,
        documentHeight: 3_697,
        bodyHeight: 3_697,
        chatPanelHeight: 3_633,
        bodyOverflow: 'visible',
      }),
    ).toBe(false);
  });
});

describe('isCombatSceneVisible', () => {
  it('accepts a cover-sized map with combatants comfortably inside the viewport', () => {
    expect(
      isCombatSceneVisible({
        viewport: { x: 0, y: 40, width: 800, height: 736 },
        map: { x: 0, y: 40, width: 800, height: 736 },
        tokens: [
          { x: 240, y: 280, width: 30, height: 30 },
          { x: 270, y: 280, width: 30, height: 30 },
        ],
      }),
    ).toBe(true);
  });

  it('rejects the old top-left spawn that was technically rendered but visually lost', () => {
    expect(
      isCombatSceneVisible({
        viewport: { x: 0, y: 40, width: 800, height: 736 },
        map: { x: 0, y: 40, width: 800, height: 736 },
        tokens: [
          { x: 30, y: 40, width: 30, height: 30 },
          { x: 90, y: 40, width: 30, height: 30 },
        ],
      }),
    ).toBe(false);
  });

  it('rejects a map that is letterboxed instead of filling the VTT', () => {
    expect(
      isCombatSceneVisible({
        viewport: { x: 0, y: 40, width: 800, height: 736 },
        map: { x: 150, y: 120, width: 500, height: 500 },
        tokens: [{ x: 300, y: 300, width: 30, height: 30 }],
      }),
    ).toBe(false);
  });
});

function completeEvidence(): CampaignEvidence[] {
  return [
    { checkpoint: 'tauri_present' },
    { checkpoint: 'backend_ready', port: 31_337 },
    { checkpoint: 'character_created', characterId: 'pc-1' },
    { checkpoint: 'assistant_reply_completed' },
    {
      checkpoint: 'bundled_illustration',
      source: 'bundled',
      assetId: 'illustration-tavern-01',
    },
    { checkpoint: 'bundled_map', source: 'bundled', assetId: 'map-ruins-01' },
    { checkpoint: 'vtt_visible' },
    { checkpoint: 'combat_started', revision: 4 },
    { checkpoint: 'move_requested', revision: 4 },
    { checkpoint: 'combat_revision_advanced', revision: 5 },
    { checkpoint: 'combat_acted', revision: 6 },
    { checkpoint: 'combat_turn_advanced', revision: 7 },
    { checkpoint: 'combat_ended', revision: 8 },
    { checkpoint: 'npc_observed', npcId: 'npc-1' },
    { checkpoint: 'journal_observed', journalId: 'journal-1' },
    { checkpoint: 'save_created', saveId: 'save-1' },
    { checkpoint: 'save_restored', saveId: 'save-1' },
  ];
}

describe('evaluateCampaignEvidence', () => {
  it('accepts a complete, ordered real campaign with authoritative revisions', () => {
    expect(evaluateCampaignEvidence(completeEvidence())).toEqual({
      ok: true,
      completed: REQUIRED_CAMPAIGN_CHECKPOINTS,
    });
  });

  it('rejects missing evidence instead of treating a partial campaign as passing', () => {
    const evidence = completeEvidence().filter(({ checkpoint }) => checkpoint !== 'vtt_visible');

    expect(evaluateCampaignEvidence(evidence)).toMatchObject({
      ok: false,
      code: 'out_of_order',
      expected: 'vtt_visible',
      received: 'combat_started',
    });
  });

  it('rejects checkpoints that arrive out of order', () => {
    const evidence = completeEvidence();
    [evidence[3], evidence[4]] = [evidence[4], evidence[3]];

    expect(evaluateCampaignEvidence(evidence)).toMatchObject({
      ok: false,
      code: 'out_of_order',
      expected: 'assistant_reply_completed',
      received: 'bundled_illustration',
    });
  });

  it('rejects a movement acknowledgement without a newer authoritative revision', () => {
    const evidence = completeEvidence();
    evidence[9] = { checkpoint: 'combat_revision_advanced', revision: 4 };

    expect(evaluateCampaignEvidence(evidence)).toMatchObject({
      ok: false,
      code: 'invalid_evidence',
      checkpoint: 'combat_revision_advanced',
    });
  });

  it('rejects non-bundled fallback media and duplicate asset IDs', () => {
    const generated = completeEvidence();
    generated[4] = {
      checkpoint: 'bundled_illustration',
      source: 'generated',
      assetId: 'illustration-tavern-01',
    };
    expect(evaluateCampaignEvidence(generated)).toMatchObject({
      ok: false,
      code: 'invalid_evidence',
      checkpoint: 'bundled_illustration',
    });

    const duplicate = completeEvidence();
    duplicate[5] = {
      checkpoint: 'bundled_map',
      source: 'bundled',
      assetId: 'illustration-tavern-01',
    };
    expect(evaluateCampaignEvidence(duplicate)).toMatchObject({
      ok: false,
      code: 'invalid_evidence',
      checkpoint: 'bundled_map',
    });
  });

  it('rejects restore evidence for a different save', () => {
    const evidence = completeEvidence();
    evidence[16] = { checkpoint: 'save_restored', saveId: 'save-2' };

    expect(evaluateCampaignEvidence(evidence)).toMatchObject({
      ok: false,
      code: 'invalid_evidence',
      checkpoint: 'save_restored',
    });
  });
});

describe('parseCapturedAgentEvents', () => {
  it('extracts only safe metadata and never retains prose or base64 media', () => {
    const parsed = parseCapturedAgentEvents(
      [
        'event: text_delta\ndata: {"text":"private narration"}',
        'event: image_generated\ndata: {"kind":"map","source":"bundled","asset_id":"map-1","image_b64":"SECRET"}',
        'event: tool_call_result\ndata: {"tool_name":"journal_append","is_error":false,"result":{"entry_id":"entry-1"},"args":{"entry_html":"PRIVATE"}}',
        'event: agent_done\ndata: {"total_rounds":1}',
      ].join('\n\n'),
    );

    expect(parsed).toEqual([
      { event: 'image_generated', kind: 'map', source: 'bundled', assetId: 'map-1' },
      {
        event: 'tool_call_result',
        toolName: 'journal_append',
        isError: false,
        result: { entry_id: 'entry-1' },
      },
      { event: 'agent_done' },
    ]);
    expect(JSON.stringify(parsed)).not.toContain('private narration');
    expect(JSON.stringify(parsed)).not.toContain('SECRET');
    expect(JSON.stringify(parsed)).not.toContain('PRIVATE');
  });
});

describe('isManualHeroButtonLabel', () => {
  it.each([
    'Build from scratch',
    'Create manually (advanced mode)',
    'Создать с нуля',
    'Создать вручную (расширенный режим)',
  ])('recognizes the resumable hero-wizard action: %s', (label) => {
    expect(isManualHeroButtonLabel(label)).toBe(true);
  });

  it('does not mistake the back action for hero creation', () => {
    expect(isManualHeroButtonLabel('Назад')).toBe(false);
  });
});
