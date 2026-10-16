import { describe, expect, it } from 'vitest';
import en from './en/agent.json';
import ru from './ru/agent.json';

describe('agent locale parity', () => {
  it('keeps the bundled provenance label in both locale catalogs', () => {
    expect(Object.keys(ru).sort()).toEqual(Object.keys(en).sort());
    expect(en.image_source_bundled).toBe('From the built-in collection');
    expect(ru.image_source_bundled).toBe('Из встроенной коллекции');
  });
});
