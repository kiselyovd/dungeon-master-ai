import { describe, expect, it } from 'vitest';
import ruWizard from '../locales/ru/wizard.json';

describe('ru/wizard.json locale corrections (E10)', () => {
  it('locked key is translated to Russian', () => {
    expect(ruWizard.locked).toBe('закреплено');
  });

  it('lb key is "фунт" (pounds), not "фт" (feet)', () => {
    expect(ruWizard.lb).toBe('фунт');
  });
});
