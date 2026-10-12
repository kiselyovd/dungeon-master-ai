import type { AgentRecoveryProjection } from '../../../api/contracts/agentRecovery';

export interface RecoveryView {
  requestId: string;
  actorId: string;
  model: string | null;
  phase: AgentRecoveryProjection['phase'];
  action: AgentRecoveryProjection['allowed_action'];
  progressKnown: boolean;
  totalCommands: number | null;
  finishedCommands: number | null;
  performedAttempts: number | null;
  refusedAttempts: number | null;
  clarificationId: string | null;
}

export function toRecoveryView(projection: AgentRecoveryProjection): RecoveryView {
  return {
    requestId: projection.scope.request_id,
    actorId: projection.scope.actor_id,
    model: projection.model,
    phase: projection.phase,
    action: projection.allowed_action,
    progressKnown: projection.progress_known,
    totalCommands: projection.progress_known ? projection.commands.length : null,
    finishedCommands: projection.progress_known
      ? projection.commands.filter(
          (entry) => entry.state === 'finished' || entry.state === 'skipped',
        ).length
      : null,
    performedAttempts: projection.progress_known
      ? projection.confirmed_effects.filter((entry) => entry.outcome.kind === 'performed').length
      : null,
    refusedAttempts: projection.progress_known
      ? projection.confirmed_effects.filter((entry) => entry.outcome.kind === 'refused').length
      : null,
    clarificationId: projection.clarification?.id ?? null,
  };
}
