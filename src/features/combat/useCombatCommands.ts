import { useCallback } from 'react';
import { useStore } from '../../state/useStore';
import { type CombatCommand, sendCombatCommand } from './model/commands';

let commandSequence = 1;

export function useCombatCommands() {
  const pending = useStore((state) => Object.keys(state.combat.pendingCommands).length > 0);

  const execute = useCallback(async (command: CombatCommand) => {
    const combat = useStore.getState().combat;
    if (!combat.encounterId || combat.revision < 0) return;
    const commandId = `combat-${commandSequence++}`;
    combat.markCommandPending(commandId, command.kind);
    console.debug('[combat.command]', {
      commandId,
      encounterId: combat.encounterId,
      actionKind: command.kind,
      currentRevision: combat.revision,
      transition: 'pending',
    });
    try {
      const projection = await sendCombatCommand({
        encounterId: combat.encounterId,
        revision: combat.revision,
        commandId,
        command,
      });
      const accepted = useStore.getState().combat.replaceProjection(projection);
      console.debug('[combat.command]', {
        commandId,
        encounterId: combat.encounterId,
        actionKind: command.kind,
        currentRevision: combat.revision,
        incomingRevision: projection.revision,
        transition: accepted ? 'reconciled' : 'stale',
      });
    } catch {
      console.warn('[combat.command]', {
        code: 'combat_command_rejected',
        commandId,
        encounterId: combat.encounterId,
        actionKind: command.kind,
        currentRevision: combat.revision,
      });
    } finally {
      useStore.getState().combat.settleCommand(commandId);
    }
  }, []);

  const move = useCallback(
    (combatantId: string, x: number, y: number) => execute({ kind: 'move', combatantId, x, y }),
    [execute],
  );
  const endTurn = useCallback(() => {
    const combatantId = useStore.getState().combat.currentTurnId;
    if (combatantId) return execute({ kind: 'end_turn', combatantId });
  }, [execute]);
  const cast = useCallback(() => {
    const combatantId = useStore.getState().combat.currentTurnId;
    if (combatantId) return execute({ kind: 'cast', combatantId });
  }, [execute]);
  const attack = useCallback(
    (targetId: string, attackModifier: number, damageDice: string, damageType: string) => {
      const attackerId = useStore.getState().combat.currentTurnId;
      if (attackerId) {
        return execute({
          kind: 'attack',
          attackerId,
          targetId,
          attackModifier,
          damageDice,
          damageType,
        });
      }
    },
    [execute],
  );

  return { pending, move, endTurn, cast, attack };
}
