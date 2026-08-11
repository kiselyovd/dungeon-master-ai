import type { ReactNode } from 'react';
import { ErrorBoundary } from '../components/ErrorBoundary';

export function AppOverlays({ children }: { children: ReactNode }) {
  return <ErrorBoundary level="overlay">{children}</ErrorBoundary>;
}
