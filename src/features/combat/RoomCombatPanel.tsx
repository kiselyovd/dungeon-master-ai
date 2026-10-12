import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { InferOutput } from 'valibot';
import type { BattleMap } from '../../api/contracts/battleMap';
import type { MediaReference } from '../../api/contracts/mediaAsset';
import type { HeroIntentSchema } from '../../api/contracts/roomOperations';
import type { PlayerTokenPresentation } from '../../components/CombatToken';
import { InitiativeTracker } from '../../components/InitiativeTracker';
import { VttCanvas } from '../../components/VttCanvas';
import { Button } from '../../ui/Button';
import { parseCombatProjection, projectionTokens } from './model/combatProjection';
import { SpellCastDialog } from './SpellCastDialog';
import { WeaponAttackDialog } from './WeaponAttackDialog';

interface Props {
  mapMediaRef?: MediaReference | null;
  timelineReset?: number;
  raw: unknown;
  map: BattleMap | null;
  player: PlayerTokenPresentation;
  disabled: boolean;
  onAction: (
    encounter: string,
    revision: number,
    action: InferOutput<typeof HeroIntentSchema>,
  ) => Promise<boolean>;
}
export function RoomCombatPanel({
  mapMediaRef,
  raw,
  map,
  player,
  disabled,
  onAction,
  timelineReset = 0,
}: Props) {
  const { t } = useTranslation('combat');
  const projection = raw === null ? null : parseCombatProjection(raw);
  const tokens = projection ? projectionTokens(projection) : [];
  const own = tokens.find((token) => token.characterId === player.characterId);
  const active = projection?.snapshot.active ?? false;
  const arena = active ? (projection?.authored?.map ?? map) : map;
  const current = projection?.snapshot.current_combatant ?? null;
  const ownsTurn = active && own !== undefined && own.id === current && own.hp > 0;
  const canAct = !disabled && ownsTurn;
  const scope = `${timelineReset}:${projection?.encounter_id}:${projection?.revision}:${own?.id}`;
  const [dialog, setDialog] = useState<{ kind: 'weapon' | 'spell'; scope: string } | null>(null);
  const act = (action: InferOutput<typeof HeroIntentSchema>) =>
    projection && canAct
      ? onAction(projection.encounter_id, projection.revision, action)
      : Promise.resolve(false);
  if (raw !== null && !projection) return <p role="alert">{t('invalid_projection')}</p>;
  if (!arena && !active) return null;
  return (
    <section aria-label={t('map_controls')} style={{ position: 'relative' }}>
      <div
        style={{
          position: 'relative',
          height: Math.max((arena?.height ?? 20) * 30, (tokens.length + 2) * 60),
          overflow: 'hidden',
        }}
      >
        <VttCanvas
          presentation={{
            mapMediaRef: mapMediaRef ?? null,
            mediaScope: player.mediaScope ?? null,
            active,
            tokens,
            map: arena ?? null,
            currentTurnId: current,
            revision: projection?.revision ?? 0,
            player,
            movableTokenId: canAct ? (own?.id ?? null) : null,
          }}
          onMoveToken={(id, x, y) => {
            if (id === own?.id) void act({ kind: 'move', x, y });
          }}
        />
        {active && projection && (
          <InitiativeTracker
            tokens={tokens}
            order={projection.snapshot.initiative.map((entry) => entry.id)}
            round={projection.snapshot.round}
            activeTokenId={current}
          />
        )}
      </div>
      {active && (
        <div className="dm-action-bar" role="toolbar" aria-label={t('action_bar')}>
          <Button
            disabled={!canAct || !own?.actionAvailable}
            onClick={() => setDialog({ kind: 'weapon', scope })}
          >
            {t('attack')}
          </Button>
          <Button
            disabled={!canAct || !own?.casting}
            onClick={() => setDialog({ kind: 'spell', scope })}
          >
            {t('cast')}
          </Button>
          <Button disabled={!canAct} onClick={() => void act({ kind: 'end_turn' })}>
            {t('end_turn')}
          </Button>
          <Button disabled={!canAct} onClick={() => void act({ kind: 'retreat' })}>
            {t('retreat_room')}
          </Button>
        </div>
      )}
      {ownsTurn && own && dialog?.scope === scope && dialog.kind === 'weapon' && (
        <WeaponAttackDialog
          key={scope}
          actor={own}
          disabled={disabled}
          targets={tokens.filter((token) => !token.characterId && token.hp > 0)}
          onClose={() => setDialog(null)}
          onAttack={(target, weapon_id, mode, two_handed) =>
            act({ kind: 'weapon', target, weapon_id, mode, two_handed })
          }
        />
      )}
      {ownsTurn && own && dialog?.scope === scope && dialog.kind === 'spell' && (
        <SpellCastDialog
          key={scope}
          actor={own}
          disabled={disabled}
          targets={tokens}
          onClose={() => setDialog(null)}
          onCast={(spell_id, targets, slot_level) =>
            act({ kind: 'spell', spell_id, targets, slot_level })
          }
        />
      )}
    </section>
  );
}
