import type { CombatToken as TokenData } from '../state/combat';

interface Props {
  token: TokenData;
  cellSize: number;
  onMove?: (id: string, x: number, y: number) => void;
}

function hpColor(hp: number, maxHp: number): string {
  const pct = maxHp > 0 ? hp / maxHp : 0;
  if (pct > 0.5) return 'var(--color-success)';
  if (pct > 0.25) return 'var(--color-warning)';
  return 'var(--color-danger)';
}

/**
 * CombatToken renders as a positioned HTML overlay above the VTT canvas.
 * Using HTML/CSS rather than PixiJS primitives keeps the token testable in
 * jsdom and lets the design tokens flow in via CSS custom properties.
 *
 * Design spec:
 * - Pulsing gold ring on active turn: 1.6s scale + opacity loop via CSS.
 * - AC chip overlay: top-right, 18x18 round, mono 11px.
 * - Status-icon ring: max 3 visible + "+N" badge.
 * - HP bar color by percent: success >50%, warning 25-50%, danger <25%.
 */
export function CombatToken({ token, cellSize }: Props) {
  const px = token.x * cellSize;
  const py = token.y * cellSize;
  const hpPct = token.maxHp > 0 ? (token.hp / token.maxHp) * 100 : 0;
  const visibleConditions = token.conditions.slice(0, 3);
  const extraConditions = token.conditions.length > 3 ? token.conditions.length - 3 : 0;

  return (
    <div
      data-testid={`combat-token-${token.id}`}
      data-active={token.isActive ? 'true' : undefined}
      style={{
        position: 'absolute',
        left: px,
        top: py,
        width: cellSize,
        height: cellSize,
        animation: token.isActive ? 'token-pulse 1.6s ease-in-out infinite' : undefined,
      }}
    >
      <div
        style={{
          width: cellSize - 4,
          height: cellSize - 4,
          borderRadius: '50%',
          background: 'var(--color-bg-raised)',
          border: token.isActive
            ? '2px solid var(--color-accent)'
            : '1px solid var(--color-border-subtle)',
          boxShadow: token.isActive ? 'var(--glow-accent)' : undefined,
          position: 'relative',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: 'var(--text-xs)',
          color: 'var(--color-fg-primary)',
        }}
      >
        {token.name.charAt(0).toUpperCase()}

        <div
          data-testid={`combat-token-${token.id}-ac`}
          style={{
            position: 'absolute',
            top: -4,
            right: -4,
            width: 18,
            height: 18,
            borderRadius: '50%',
            background: 'var(--color-bg-deep)',
            border: '1px solid var(--color-border-strong)',
            fontSize: 'var(--text-xs)',
            fontFamily: 'var(--font-mono)',
            letterSpacing: 'var(--tracking-mono)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: 'var(--color-fg-secondary)',
          }}
        >
          {token.ac}
        </div>

        {visibleConditions.length > 0 && (
          <div
            style={{
              position: 'absolute',
              bottom: -8,
              left: 0,
              right: 0,
              display: 'flex',
              justifyContent: 'center',
              gap: 2,
            }}
          >
            {visibleConditions.map((c) => (
              <span
                key={c}
                title={c}
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: '50%',
                  background: 'var(--color-warning)',
                }}
              />
            ))}
            {extraConditions > 0 && (
              <span
                style={{
                  fontSize: 8,
                  color: 'var(--color-fg-muted)',
                  lineHeight: '8px',
                }}
              >
                +{extraConditions}
              </span>
            )}
          </div>
        )}
      </div>

      <div
        style={{
          position: 'absolute',
          bottom: 0,
          left: 0,
          right: 0,
          height: 3,
          background: 'var(--color-bg-deep)',
          borderRadius: 2,
          overflow: 'hidden',
        }}
      >
        <div
          data-testid={`combat-token-${token.id}-hpbar`}
          style={{
            width: `${hpPct}%`,
            height: '100%',
            background: hpColor(token.hp, token.maxHp),
            transition: 'width var(--t-base) ease',
          }}
        />
      </div>
    </div>
  );
}
