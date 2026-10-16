import { createContext, type ReactNode, useContext } from 'react';
import type { TablePresentation, TablePresentationPorts } from './model/types';

const Context = createContext<{ model: TablePresentation; ports: TablePresentationPorts } | null>(
  null,
);
export function TableContext({
  model,
  ports,
  children,
}: {
  model: TablePresentation;
  ports: TablePresentationPorts;
  children: ReactNode;
}) {
  return <Context.Provider value={{ model, ports }}>{children}</Context.Provider>;
}
export function useTablePresentation() {
  const value = useContext(Context);
  if (!value) throw new Error('table_context_missing');
  return value;
}
