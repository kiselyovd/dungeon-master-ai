import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ChatError } from '../errors';
import { fetchCompendium, resetSrdCacheForTests } from '../srd';

vi.mock('../client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://test.local${path}`),
}));

const fixtures: Record<string, unknown> = {
  '/srd/races': [
    {
      id: 'dwarf',
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
      srd_section: 'SRD 5.1',
    },
  ],
  '/srd/classes': [],
  '/srd/backgrounds': [],
  '/srd/spells': [
    {
      id: 'fire-bolt',
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
      srd_section: 'SRD 5.1',
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
});
