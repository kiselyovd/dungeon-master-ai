import { afterEach, expect, it } from 'vitest';
import { backendUrl, setBackendPortForTesting } from './client';

afterEach(() => setBackendPortForTesting(null));

it('keeps WebView requests on the localhost site boundary', async () => {
  setBackendPortForTesting(45678);

  await expect(backendUrl('/health')).resolves.toBe('http://localhost:45678/health');
});
