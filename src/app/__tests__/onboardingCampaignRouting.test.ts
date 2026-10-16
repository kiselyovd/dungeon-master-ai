// @vitest-environment node
import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

describe('onboarding campaign routing', () => {
  it('does not retain the legacy wizard reopen callback behind campaign home', () => {
    const appSource = readFileSync(new URL('../App.tsx', import.meta.url), 'utf8');
    const onboardingSource = readFileSync(
      new URL('../../components/Onboarding.tsx', import.meta.url),
      'utf8',
    );

    expect(appSource).not.toContain('onExitToWizard');
    expect(onboardingSource).not.toContain('onExitToWizard');
  });

  it('uses the character projection as the only new-campaign hero gate', () => {
    const appSource = readFileSync(new URL('../App.tsx', import.meta.url), 'utf8');

    expect(appSource).not.toContain('campaignNeedsHero');
    expect(appSource).toContain('useStore.getState().pc.setHeroClass(null)');
  });
});
