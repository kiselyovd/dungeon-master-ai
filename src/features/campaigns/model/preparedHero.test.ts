import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import type { HeroClassId } from '../../../state/pc';
import { preparedHero } from './preparedHero';

it('matches the server fixture and selects only class-legal spells from the actual SRD catalog', () => {
  const catalog = JSON.parse(
    readFileSync('crates/app-domain/src/compendium/data/srd_5_2_1.yaml', 'utf8'),
  );
  const fixtures = JSON.parse(
    readFileSync('crates/adapter-sqlite/tests/fixtures/prepared_heroes.json', 'utf8'),
  );
  for (const classId of ['fighter', 'rogue', 'wizard', 'cleric'] as HeroClassId[]) {
    const hero = preparedHero(classId);
    expect(hero).toEqual(fixtures[classId]);
    for (const spellId of [...hero.spells.cantrips, ...hero.spells.level1]) {
      const spell = catalog.spells.find((entry: { id: string }) => entry.id === spellId);
      expect(spell?.classes).toContain(classId);
    }
    expect(hero.hp).toBeGreaterThan(0);
    expect(hero.inventory.some((item) => item.id.startsWith(`${classId}-`))).toBe(false);
  }
});
