import { describe, expect, it, vi } from 'vitest';

import { discardResponseBody } from './response';

describe('discardResponseBody', () => {
  it('drains the response instead of aborting its reusable WebView2 connection', async () => {
    const arrayBuffer = vi.fn(async () => new ArrayBuffer(0));
    const cancel = vi.fn(async () => undefined);
    const response = {
      arrayBuffer,
      body: { cancel },
    } as unknown as Response;

    await discardResponseBody(response);

    expect(arrayBuffer).toHaveBeenCalledOnce();
    expect(cancel).not.toHaveBeenCalled();
  });
});
