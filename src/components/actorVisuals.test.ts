import { expect, it } from 'vitest';
import { actorBuiltinAppearance } from './actorVisuals';

it('keeps the selected portrait and matching whole miniature as distinct resources', () => {
  const art = actorBuiltinAppearance({
    characterId: 'actor',
    heroClass: 'fighter',
    portraitAssetId: 'hero.fighter.01.v2',
  });
  expect(art.miniatureId).toBe('miniature.fighter.01.v2');
  expect(art.miniature).not.toBe(art.portrait);
});
it('does not invent a matching miniature or change an unknown assigned portrait', () => {
  expect(
    actorBuiltinAppearance({
      characterId: 'actor',
      heroClass: 'fighter',
      portraitAssetId: 'unknown.assigned.face',
    }),
  ).toMatchObject({ portrait: null, miniature: null, assigned: true });
  expect(
    actorBuiltinAppearance({
      characterId: 'actor',
      heroClass: 'fighter',
      portraitUrl: 'data:image/png;base64,legacy',
    }).miniature,
  ).toBeNull();
});
it('uses only the stable legacy class identity when no appearance has been assigned', () => {
  expect(actorBuiltinAppearance({ characterId: 'actor', heroClass: 'rogue' }).miniatureId).toBe(
    'miniature.rogue.legacy.v1',
  );
});
