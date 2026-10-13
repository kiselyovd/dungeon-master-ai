import { type ReactNode, useEffect } from 'react';
import { tableScopeKey } from './model/layout';
import type { TablePresentation } from './model/types';
import styles from './TableSurface.module.css';

/** The common scope surface preserves mounted controls and unsent drafts when
 * timeline/encounter metadata changes. Selection invalidation uses its scope key. */
export function TableSurface({
  model,
  children,
}: {
  model: TablePresentation;
  children: ReactNode;
}) {
  const scopeKey = tableScopeKey(model.scope);
  useEffect(() => {
    if (import.meta.env.VITE_TABLE_TRACE === '1') console.debug('[table] scope', { key: scopeKey });
  }, [scopeKey]);
  return (
    <div
      className={styles.scope}
      data-game-table
      data-table-mode={model.scope.mode}
      data-table-scope={scopeKey}
      data-table-scene={model.scene.kind}
      data-own-actor={model.ownActorId ?? ''}
      data-table-readiness={model.readiness}
    >
      {children}
    </div>
  );
}
