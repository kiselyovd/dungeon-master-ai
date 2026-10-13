import type { ReactNode } from 'react';
import {
  TableContext,
  type TablePresentation,
  type TablePresentationPorts,
  TableSurface,
} from '../features/game-table';

/** Production composition only; role controllers keep workflows and authority. */
export function GameTableExperience({
  model,
  ports,
  children,
}: {
  model: TablePresentation;
  ports: TablePresentationPorts;
  children: ReactNode;
}) {
  return (
    <TableContext model={model} ports={ports}>
      <TableSurface model={model}>{children}</TableSurface>
    </TableContext>
  );
}
