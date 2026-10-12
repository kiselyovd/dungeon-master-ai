import type { CSSProperties, ReactNode } from 'react';
import { DEFAULT_READING_FONT_SIZE, type ReadingFontSize } from '../state/settings';

export function AppShell({
  children,
  readingFontSize = DEFAULT_READING_FONT_SIZE,
}: {
  children: ReactNode;
  readingFontSize?: ReadingFontSize;
}) {
  return (
    <div className="dm-app" style={{ '--text-reading': `${readingFontSize}px` } as CSSProperties}>
      {children}
    </div>
  );
}
