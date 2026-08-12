// @vitest-environment node

import { describe, expect, it } from 'vitest';
import { evaluateDmReply } from './dm-reply';

describe('evaluateDmReply', () => {
  it('keeps waiting for reasoning-only or a streaming partial', () => {
    expect(evaluateDmReply({ assistantMessages: ['old'], isStreaming: true }, 1)).toEqual({
      status: 'pending',
    });
    expect(
      evaluateDmReply({ assistantMessages: ['old', 'partial'], isStreaming: true }, 1),
    ).toEqual({ status: 'pending' });
  });

  it('accepts only a new completed assistant message', () => {
    expect(
      evaluateDmReply({ assistantMessages: ['old', 'The door opens.'], isStreaming: false }, 1),
    ).toEqual({ status: 'complete', text: 'The door opens.' });
  });

  it('keeps waiting when the new completed bubble is empty', () => {
    expect(
      evaluateDmReply({ assistantMessages: ['old', '   '], isStreaming: false }, 1),
    ).toEqual({ status: 'pending' });
    expect(
      evaluateDmReply({ assistantMessages: ['old', '…'], isStreaming: false }, 1),
    ).toEqual({ status: 'pending' });
  });

  it('surfaces an agent error instead of timing out', () => {
    expect(
      evaluateDmReply(
        { assistantMessages: [], isStreaming: false, errorCode: 'provider_error' },
        0,
      ),
    ).toEqual({ status: 'error', code: 'provider_error' });
  });
});
