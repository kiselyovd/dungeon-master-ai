import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ChatError } from '../errors';
import { CURRENT_RULESET_ID, fetchCompendium, resetSrdCacheForTests } from '../srd';

vi.mock('../client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://test.local${path}`),
}));

const fixtures: Record<string, unknown> = {
  '/srd/races': [
    {
      id: 'dwarf',
      ruleset_id: CURRENT_RULESET_ID,
      name_key: 'catalog.species.dwarf',
      name_en: 'Dwarf',
      name_ru: 'Дварф',
      ability_score_increases: { con: 2 },
      age: { mature_at: 50, max_lifespan: 350 },
      size: 'Medium',
      speed: 25,
      languages: ['Common', 'Dwarvish'],
      proficiencies: { skills: [], weapons: [], tools: [], saves: [] },
      senses: { darkvision_ft: 60 },
      traits: [],
      subraces: [],
      source_url: 'https://example.test/',
      source_page: 84,
      srd_section: 'Character Origins / Dwarf',
    },
  ],
  '/srd/classes': [],
  '/srd/backgrounds': [],
  '/srd/spells': [
    {
      id: 'fire-bolt',
      ruleset_id: CURRENT_RULESET_ID,
      name_key: 'catalog.spells.fire-bolt',
      name_en: 'Fire Bolt',
      name_ru: 'Огненный снаряд',
      level: 0,
      school: 'evocation',
      casting_time: '1 action',
      range_ft: 120,
      components: { v: true, s: true },
      duration: 'Instantaneous',
      ritual: false,
      concentration: false,
      classes: ['wizard'],
      description_en: 'x',
      description_ru: 'ы',
      source_url: 'https://example.test/',
      source_page: 110,
      srd_section: 'Spells / Fire Bolt',
    },
  ],
  '/srd/equipment': { weapons: [], armor: [], adventuring_gear: [] },
  '/srd/feats': [],
  '/srd/weapon-properties': [],
};

beforeEach(() => {
  resetSrdCacheForTests();
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString();
      const path = new URL(url).pathname;
      const body = fixtures[path];
      if (body === undefined) {
        return new Response('not found', { status: 404 });
      }
      return new Response(JSON.stringify(body), {
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
      });
    }),
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
  resetSrdCacheForTests();
});

describe('fetchCompendium', () => {
  it('fans out across all seven endpoints and parses each response', async () => {
    const c = await fetchCompendium();
    expect(c.rulesetId).toBe(CURRENT_RULESET_ID);
    expect(c.races).toHaveLength(1);
    expect(c.races[0]?.id).toBe('dwarf');
    expect(c.spells[0]?.id).toBe('fire-bolt');
    expect(c.equipment).toEqual({ weapons: [], armor: [], adventuring_gear: [] });
    expect(c.feats).toEqual([]);
    expect(c.weapon_properties).toEqual([]);
  });

  it('caches the first call (does not refetch on subsequent calls)', async () => {
    const fetchSpy = global.fetch as unknown as ReturnType<typeof vi.fn>;
    await fetchCompendium();
    await fetchCompendium();
    expect(fetchSpy).toHaveBeenCalledTimes(7);
  });

  it('throws ChatError when an endpoint returns non-2xx', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('', { status: 500 })),
    );
    await expect(fetchCompendium()).rejects.toBeInstanceOf(ChatError);
  });

  it('rejects a catalog entry from another ruleset', async () => {
    const original = fixtures['/srd/races'];
    const first = (original as Array<Record<string, unknown>>)[0] ?? {};
    fixtures['/srd/races'] = [{ ...first, ruleset_id: 'srd-5.1' }];
    await expect(fetchCompendium()).rejects.toMatchObject({ code: 'invalid_response' });
    fixtures['/srd/races'] = original;
  });

  it('clears the cache on failure so a retry re-fetches (E5)', async () => {
    // First attempt fails - the rejected promise must NOT be cached.
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('', { status: 500 })),
    );
    await expect(fetchCompendium()).rejects.toBeInstanceOf(ChatError);

    // Retry against a healthy backend (no resetSrdCacheForTests between calls):
    // it only succeeds if fetchCompendium cleared its own cache on rejection.
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) => {
        const url = typeof input === 'string' ? input : input.toString();
        const body = fixtures[new URL(url).pathname];
        return body === undefined
          ? new Response('not found', { status: 404 })
          : new Response(JSON.stringify(body), {
              status: 200,
              headers: new Headers({ 'content-type': 'application/json' }),
            });
      }),
    );
    const c = await fetchCompendium();
    expect(c.races[0]?.id).toBe('dwarf');
  });
});
