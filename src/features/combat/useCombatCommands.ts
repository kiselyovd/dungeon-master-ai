import { useCallback, useRef } from 'react';
import { applyCombatOutcome } from '../../state/combatOutcome';
import { useStore } from '../../state/useStore';
import { type CombatCommand, sendCombatCommand } from './model/commands';

export function useCombatCommands() {
  const pending = useStore((state) => Object.keys(state.combat.pendingCommands).length > 0);
  const retry = useRef<{
    signature: string;
    input: Parameters<typeof sendCombatCommand>[0];
  } | null>(null);

  const execute = useCallback(async (command: CombatCommand) => {
    const state = useStore.getState();
    const combat = state.combat;
    if (!combat.encounterId || combat.revision < 0) return false;
    if (state.pc.characterId) {
      const owned = combat.tokens.find((token) => token.characterId === state.pc.characterId);
      const actorId = command.kind === 'attack' ? command.attackerId : command.combatantId;
      if (!owned || owned.id !== actorId || combat.currentTurnId !== owned.id) return false;
    }
    if (state.session.mutationId || Object.keys(combat.pendingCommands).length > 0) return false;
    const scope = {
      campaignId: state.session.activeCampaignId,
      sessionId: state.session.activeSessionId,
      generation: state.session.transitionGeneration,
    };
    const signature = JSON.stringify({
      ...scope,
      encounterId: combat.encounterId,
      revision: combat.revision,
      command,
    });
    const input =
      retry.current?.signature === signature
        ? retry.current.input
        : {
            encounterId: combat.encounterId,
            revision: combat.revision,
            commandId: crypto.randomUUID(),
            command: structuredClone(command),
          };
    retry.current = { signature, input };
    const commandId = input.commandId;
    const isCurrent = () => {
      const current = useStore.getState();
      return (
        current.session.activeCampaignId === scope.campaignId &&
        current.session.activeSessionId === scope.sessionId &&
        current.session.transitionGeneration === scope.generation &&
        current.combat.encounterId === input.encounterId
      );
    };
    combat.markCommandPending(commandId, command.kind);
    console.debug('[combat.command]', {
      commandId,
      encounterId: combat.encounterId,
      actionKind: command.kind,
      currentRevision: combat.revision,
      transition: 'pending',
    });
    try {
      const projection = await sendCombatCommand(input);
      if (retry.current?.input.commandId === commandId) retry.current = null;
      if (!isCurrent() || projection.encounter_id !== input.encounterId) return false;
      const accepted = applyCombatOutcome(projection);
      console.debug('[combat.command]', {
        commandId,
        encounterId: combat.encounterId,
        actionKind: command.kind,
        currentRevision: combat.revision,
        incomingRevision: projection.revision,
        transition: accepted ? 'reconciled' : 'stale',
      });
      return accepted;
    } catch {
      console.warn('[combat.command]', {
        code: 'combat_command_rejected',
        commandId,
        encounterId: combat.encounterId,
        actionKind: command.kind,
        currentRevision: combat.revision,
      });
      return false;
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
  const weaponAttack = useCallback(
    (
      targetId: string,
      weaponId: string,
      mode: 'default' | 'melee' | 'thrown' | 'ranged',
      twoHanded: boolean,
    ) => {
      const attackerId = useStore.getState().combat.currentTurnId;
      return attackerId
        ? execute({
            kind: 'attack',
            attackerId,
            targetId,
            weaponId,
            mode,
            twoHanded,
            attackModifier: 0,
            damageDice: '',
            damageType: '',
          })
        : Promise.resolve(false);
    },
    [execute],
  );

  return { pending, move, endTurn, cast, attack, weaponAttack };
}
