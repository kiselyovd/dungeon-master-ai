export interface UnconfirmedDeparture {
  kind: 'unconfirmed_departure';
  preserveAndExit(): Promise<void>;
}
export interface ClosePolicy {
  scope: string;
  kind: 'idle' | 'solo' | 'host' | 'guest';
  pending: boolean;
  prepare(): Promise<void> | Promise<UnconfirmedDeparture | undefined>;
}
export function requiresCloseConfirmation(policy: ClosePolicy): boolean {
  return policy.pending || policy.kind === 'host' || policy.kind === 'guest';
}
export function sameCloseScope(preview: ClosePolicy, current: ClosePolicy): boolean {
  return preview.scope === current.scope && preview.kind === current.kind;
}
