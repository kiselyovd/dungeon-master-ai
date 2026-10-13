import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { InferOutput } from 'valibot';
import type { BattleMap } from '../../api/contracts/battleMap';
import type { MediaReference } from '../../api/contracts/mediaAsset';
import type { HeroIntentSchema } from '../../api/contracts/roomOperations';
import { ActionBar } from '../../components/ActionBar';
import type { ActorVisualMap } from '../../components/actorVisuals';
import type { PlayerTokenPresentation } from '../../components/CombatToken';
import { InitiativeTracker } from '../../components/InitiativeTracker';
import type { SceneArtworkPresentation } from '../../components/SceneArtwork';
import { VttCanvas } from '../../components/VttCanvas';
import { parseCombatProjection, projectionTokens } from './model/combatProjection';
import { canCast } from './model/spellAvailability';
import { SpellCastDialog } from './SpellCastDialog';
import { useCombatSelection } from './useCombatSelection';
import { WeaponAttackDialog } from './WeaponAttackDialog';

interface Props {
  mapSkinAssetId?: string | null;
  sceneArtwork?: SceneArtworkPresentation | null;
  actorVisuals?: ActorVisualMap | undefined;
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
  mapSkinAssetId,
  sceneArtwork,
  actorVisuals,
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
  const selection = useCombatSelection(
    `${timelineReset}:${projection?.encounter_id}:${player.characterId}`,
    tokens,
  );
  const [dialog, setDialog] = useState<{ kind: 'weapon' | 'spell'; scope: string } | null>(null);
  const act = (action: InferOutput<typeof HeroIntentSchema>) =>
    projection && canAct
      ? onAction(projection.encounter_id, projection.revision, action)
      : Promise.resolve(false);
  if (raw !== null && !projection) return <p role="alert">{t('invalid_projection')}</p>;
  if (!arena && !active && !sceneArtwork) return null;
  return (
    <section
      aria-label={t('map_controls')}
      style={{
        position: 'relative',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        minHeight: 0,
      }}
    >
      <div
        style={{
          position: 'relative',
          flex: 1,
          minHeight: 0,
          overflow: 'hidden',
        }}
      >
        <VttCanvas
          selectedTokenId={selection.selectedId}
          onSelectToken={selection.select}
          presentation={{
            mapSkinAssetId: active
              ? (projection?.authored?.map_skin_asset_id ?? null)
              : (mapSkinAssetId ?? null),
            sceneArtwork: sceneArtwork ?? null,
            actorVisuals,
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
            compact
            tokens={tokens}
            order={projection.snapshot.initiative.map((entry) => entry.id)}
            round={projection.snapshot.round}
            activeTokenId={current}
            selectedTokenId={selection.selectedId}
            onSelect={selection.select}
          />
        )}
        {active && (
          <ActionBar
            supportedOnly
            blockedReason={!canAct ? t('action_turn_unavailable') : null}
            actionUsed={!own?.actionAvailable}
            bonusUsed={!own?.bonusAvailable}
            reactionUsed={!own?.reactionAvailable}
            movementFt={own?.movementRemaining ?? 0}
            speedFt={own?.speed ?? 0}
            slots={own?.casting?.slots}
            castAvailable={Boolean(own?.casting && canCast(own))}
            onAttack={() => setDialog({ kind: 'weapon', scope })}
            onCast={own?.casting ? () => setDialog({ kind: 'spell', scope }) : undefined}
            onEndTurn={() => void act({ kind: 'end_turn' })}
            onRetreat={projection?.authored ? () => void act({ kind: 'retreat' }) : undefined}
            retreatAvailable={Boolean(
              own &&
                projection?.authored?.map.exits.some(
                  (exit) => exit.x === own.x && exit.y === own.y,
                ),
            )}
          />
        )}
      </div>
      {ownsTurn && own && dialog?.scope === scope && dialog.kind === 'weapon' && (
        <WeaponAttackDialog
          key={scope}
          actor={own}
          initialTargetId={selection.selectedId}
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
          initialTargetId={selection.selectedId}
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
