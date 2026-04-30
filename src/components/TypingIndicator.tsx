/**
 * Three Cinzel diamond glyphs pulsing in a 1.4s staggered loop.
 *
 * Animation uses CSS @keyframes typing-dot (defined in src/styles/combat.css)
 * with animation-delay staggered by 0.2s per diamond so they pulse sequentially.
 * Color is var(--color-accent) (gold). Diamond character is U+25C6.
 */
export function TypingIndicator() {
  return (
    <span
      aria-label="Dungeon Master is writing"
      role="status"
      style={{
        display: 'inline-flex',
        gap: 'var(--space-1)',
        alignItems: 'center',
        height: '1.5em',
      }}
    >
      {([0, 1, 2] as const).map((i) => (
        <span
          key={i}
          data-diamond={i}
          aria-hidden="true"
          style={{
            display: 'inline-block',
            color: 'var(--color-accent)',
            fontFamily: 'var(--font-display)',
            fontSize: 'var(--text-xs)',
            animation: 'typing-dot 1.4s ease-in-out infinite',
            animationDelay: `${i * 0.2}s`,
          }}
        >
          {'◆'}
        </span>
      ))}
    </span>
  );
}
