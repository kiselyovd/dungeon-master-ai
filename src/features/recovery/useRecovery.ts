import { useCallback, useEffect, useRef, useState } from 'react';
import type { AgentTurnRequest } from '../../api/contracts/agent';
import type { AgentRecoveryProjection } from '../../api/contracts/agentRecovery';
import { toRecoveryView } from './model/recovery';

export interface RecoveryScope {
  campaignId: string;
  sessionId: string;
  actorId: string;
  generation: number;
}
export interface RecoveryPorts {
  projection(scope: RecoveryScope, signal: AbortSignal): Promise<AgentRecoveryProjection | null>;
  input(
    scope: RecoveryScope,
    signal: AbortSignal,
  ): Promise<{ input: AgentTurnRequest; clarificationId?: string } | null>;
  prepare(input: AgentTurnRequest, clarificationId?: string): boolean;
  resume(input: AgentTurnRequest, clarificationId?: string): Promise<void>;
}
interface State {
  key: string;
  status: 'loading' | 'ready' | 'error';
  projection: AgentRecoveryProjection | null;
  input: Awaited<ReturnType<RecoveryPorts['input']>>;
}
function diagnostic(event: string, scope: RecoveryScope, requestId?: string) {
  if (import.meta.env.VITE_RECOVERY_TRACE === '1')
    console.debug('[recovery.probe]', {
      event,
      campaignId: scope.campaignId,
      sessionId: scope.sessionId,
      actorId: scope.actorId,
      generation: scope.generation,
      requestId,
    });
}
export function useRecovery(scope: RecoveryScope | null, ports: RecoveryPorts, streaming: boolean) {
  const key = scope
    ? `${scope.campaignId}:${scope.sessionId}:${scope.actorId}:${scope.generation}`
    : '';
  const scopeRef = useRef(scope);
  scopeRef.current = scope;
  const currentKey = useRef(key);
  currentKey.current = key;
  const sequence = useRef(0);
  const controller = useRef<AbortController | null>(null);
  const [state, setState] = useState<State>({
    key: '',
    status: 'ready',
    projection: null,
    input: null,
  });
  const probe = useCallback(async () => {
    const active = scopeRef.current;
    if (!active || streaming) return;
    controller.current?.abort();
    const abort = new AbortController();
    controller.current = abort;
    const request = ++sequence.current;
    const capturedKey = currentKey.current;
    diagnostic('started', active);
    const current = () =>
      !abort.signal.aborted && sequence.current === request && currentKey.current === capturedKey;
    setState((previous) =>
      previous.key === capturedKey
        ? { ...previous, status: 'loading' }
        : { key: capturedKey, status: 'loading', projection: null, input: null },
    );
    try {
      let projection: AgentRecoveryProjection | null = null;
      let input: State['input'] = null;
      for (let attempt = 0; attempt < 2; attempt++) {
        [projection, input] = await Promise.all([
          ports.projection(active, abort.signal),
          ports.input(active, abort.signal),
        ]);
        if (!current()) return;
        if (
          (projection === null) === (input === null) &&
          (!projection || projection.scope.request_id === input?.input.requestId)
        )
          break;
        if (attempt === 1) throw new Error('agent_recovery_snapshot_changed');
      }
      if (
        projection &&
        (projection.scope.actor_id !== active.actorId ||
          projection.scope.campaign_id !== active.campaignId ||
          projection.scope.session_id !== active.sessionId)
      )
        throw new Error('agent_recovery_scope_mismatch');
      if (input && !ports.prepare(input.input, input.clarificationId))
        throw new Error('agent_recovery_scope_changed');
      if (current()) setState({ key: capturedKey, status: 'ready', projection, input });
      if (current()) diagnostic('ready', active, projection?.scope.request_id);
    } catch {
      if (current()) diagnostic('failed', active);
      if (current()) setState((previous) => ({ ...previous, key: capturedKey, status: 'error' }));
    }
  }, [ports, streaming]);
  useEffect(() => {
    if (!key) {
      controller.current?.abort();
      sequence.current++;
      setState({ key: '', status: 'ready', projection: null, input: null });
      return;
    }
    if (!streaming) void probe();
    return () => {
      controller.current?.abort();
      sequence.current++;
    };
  }, [key, probe, streaming]);
  const current =
    state.key === key ? state : { key, status: 'loading' as const, projection: null, input: null };
  useEffect(() => {
    if (streaming || current.projection?.phase !== 'running' || current.status === 'error') return;
    const timeout = setTimeout(() => void probe(), 2500);
    return () => clearTimeout(timeout);
  }, [streaming, current.projection, current.status, probe]);
  const resume = useCallback(async () => {
    if (
      state.status !== 'ready' ||
      currentKey.current !== state.key ||
      state.projection?.allowed_action !== 'resume' ||
      !state.input
    )
      return;
    await ports.resume(state.input.input, state.input.clarificationId);
    await probe();
  }, [state, ports, probe]);
  return {
    view: current.projection ? toRecoveryView(current.projection) : null,
    status: key ? current.status : ('ready' as const),
    blocked: Boolean(key && (current.status !== 'ready' || current.input)),
    clarifying: current.projection?.allowed_action === 'clarify',
    retry: probe,
    resume,
  };
}
