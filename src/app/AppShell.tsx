import type { CSSProperties, ReactNode } from 'react';
import { usePresentationMotion } from '../hooks/usePresentationMotion';
import { DEFAULT_READING_FONT_SIZE, type ReadingFontSize } from '../state/settings';

export function AppShell({
  children,
  readingFontSize = DEFAULT_READING_FONT_SIZE,
}: {
  children: ReactNode;
  readingFontSize?: ReadingFontSize;
}) {
  const motion = usePresentationMotion();
  return (
    <div
      className="dm-app"
      data-motion-paused={!motion.active || motion.reduced ? 'true' : undefined}
      style={{ '--text-reading': `${readingFontSize}px` } as CSSProperties}
    >
      {children}
    </div>
  );
}
