import '../../i18n';
import { fireEvent, render } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { CombatToken as TokenData } from '../../state/combat';
import { opponentAppearance, opponentFrame } from '../actorVisuals';
import { CombatToken } from '../CombatToken';
import { InitiativeTracker } from '../InitiativeTracker';

const actor: TokenData = {
  id: 'enemy',
  name: 'Another label',
  hp: 6,
  maxHp: 6,
  ac: 11,
  x: 9,
  y: 1,
  conditions: ['poisoned'],
  opponentAppearance: 'water_echo',
};

it('keeps art pointer-free and anchored while selection, conditions and death remain canonical', () => {
  const onSelect = vi.fn(),
    onMove = vi.fn();
  const view = render(
    <CombatToken
      token={actor}
      cellSize={30}
      onSelect={onSelect}
      onMove={onMove}
      currentTurnId={actor.id}
    />,
  );
  const cell = view.getByTestId('combat-token-enemy');
  expect(cell.style.width).toBe('30px');
  expect(cell.style.left).toBe('270px');
  expect(cell.style.top).toBe('30px');
  expect(view.getByTestId('miniature-enemy')).toHaveAttribute(
    'data-asset-id',
    'opponent.water_echo.v2',
  );
  expect(view.getByTestId('miniature-enemy').style.pointerEvents).toBe('none');
  fireEvent.click(cell);
  expect(onSelect).toHaveBeenCalledWith('enemy');
  fireEvent.pointerDown(cell, { button: 0, pointerId: 1, clientX: 100, clientY: 100 });
  fireEvent.pointerUp(cell, { pointerId: 1, clientX: 200, clientY: 200 });
  expect(onMove).not.toHaveBeenCalled();
  view.rerender(<CombatToken token={{ ...actor, hp: 0, isDead: true }} cellSize={30} />);
  expect(view.getByTestId('combat-token-skull')).toBeVisible();
  expect(view.getByTestId('miniature-enemy')).toHaveAttribute(
    'data-asset-id',
    'opponent.water_echo.v2',
  );
  view.rerender(
    <CombatToken token={{ ...actor, opponentAppearance: 'future_opponent' }} cellSize={30} />,
  );
  expect(view.queryByTestId('miniature-enemy')).toBeNull();
  expect(opponentAppearance('https://remote.invalid')).toBeNull();
});

it.each([
  'water_echo',
  'goblin',
  'skeleton',
  'zombie',
  'wolf',
  'bandit',
  'orc',
  'giant_rat',
])('aligns %s measured feet to the same base at every cell size', (appearance) => {
  const asset = opponentAppearance(appearance);
  if (!asset) throw new Error('missing production sprite');
  for (const size of [30, 48, 64]) {
    const frame = opponentFrame(asset, size);
    const scale = frame.height / asset.height;
    expect(frame.bottom + (asset.height - asset.spriteBounds.bottom - 1) * scale).toBeCloseTo(
      size * 0.08,
    );
  }
});

it('uses the same explicit image in the initiative consumer', () => {
  const view = render(<InitiativeTracker tokens={[actor]} order={[actor.id]} round={1} />);
  expect(view.container.querySelector('img')).toHaveAttribute(
    'data-asset-id',
    'opponent.water_echo.v2',
  );
});

it('does not let the map capture a primary click on a stationary opponent', () => {
  const pan = vi.fn(),
    select = vi.fn();
  const view = render(
    <div onPointerDown={pan}>
      <CombatToken token={actor} cellSize={30} onSelect={select} />
    </div>,
  );
  fireEvent.pointerDown(view.getByTestId('combat-token-enemy'), { button: 0, pointerId: 1 });
  expect(pan).not.toHaveBeenCalled();
  fireEvent.click(view.getByTestId('combat-token-enemy'));
  expect(select).toHaveBeenCalledWith('enemy');
});
