import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Field, Modal } from '../../ui';
import { Select } from '../../ui/Select';
import { isDeadToken } from './model/selectors';
import { preferredSpellSlot, spellAvailable } from './model/spellAvailability';
import type { CombatToken, SpellChoiceDto } from './model/types';

interface Props {
  initialTargetId?: string | null | undefined;
  disabled?: boolean;
  actor: CombatToken;
  targets: CombatToken[];
  onClose: () => void;
  onCast: (spellId: string, targetIds: string[], slotLevel: number) => Promise<boolean>;
}
export function SpellCastDialog({
  actor,
  targets,
  onClose,
  onCast,
  disabled = false,
  initialTargetId,
}: Props) {
  const { t } = useTranslation('combat');
  const choices = actor.casting?.choices ?? [];
  const first = choices.find((choice) => choice.supported);
  const [spellId, setSpellId] = useState(first?.id ?? '');
  const [slot, setSlot] = useState(preferredSpellSlot(actor, first));
  const rule = choices.find((choice) => choice.id === spellId);
  const eligible = (rule: SpellChoiceDto | undefined) =>
    targets.filter((target) =>
      rule?.healing
        ? Boolean(target.characterId) && !isDeadToken(target)
        : !target.characterId && target.hp > 0,
    );
  const [targetId, setTargetId] = useState(
    eligible(first).find((target) => target.id === initialTargetId)?.id ??
      eligible(first)[0]?.id ??
      '',
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const available = Boolean(
    !disabled &&
      rule &&
      spellAvailable(actor, rule, slot) &&
      eligible(rule).some((target) => target.id === targetId),
  );
  const cast = async () => {
    if (!available || busy) return;
    setBusy(true);
    setError(false);
    try {
      if (await onCast(spellId, [targetId], slot)) onClose();
      else setError(true);
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      open
      title={t('spell_title')}
      onClose={onClose}
      disableClose={busy}
      footer={
        <>
          <Button onClick={onClose} disabled={busy}>
            {t('weapon_cancel')}
          </Button>
          <Button variant="primary" disabled={busy || !available} onClick={() => void cast()}>
            {t('spell_confirm')}
          </Button>
        </>
      }
    >
      {!available && (
        <p role="status">
          {t(
            disabled
              ? 'action_turn_unavailable'
              : !rule || !spellAvailable(actor, rule, slot)
                ? 'action_spell_resource_unavailable'
                : 'action_target_unavailable',
          )}
        </p>
      )}
      <Field label={t('spell_selection')}>
        {({ id }) => (
          <Select
            id={id}
            value={spellId}
            disabled={busy}
            options={choices.map((choice) => ({
              value: choice.id,
              label: `${t(`spell_names.${choice.id}`)}${choice.supported ? '' : ` (${t('spell_unsupported')})`}`,
              disabled: !choice.supported,
            }))}
            onChange={(value) => {
              const next = choices.find((choice) => choice.id === value);
              setSpellId(value);
              setSlot(preferredSpellSlot(actor, next));
              setTargetId(eligible(next)[0]?.id ?? '');
            }}
          />
        )}
      </Field>
      <Field label={t('weapon_target')}>
        {({ id }) => (
          <Select
            id={id}
            value={targetId}
            disabled={busy}
            options={eligible(rule).map((target) => ({
              value: target.id,
              label: `${target.displayNameKey ? t(target.displayNameKey, { defaultValue: target.name }) : target.name} (${target.hp}/${target.maxHp})`,
            }))}
            onChange={setTargetId}
          />
        )}
      </Field>
      {rule && rule.level > 0 && (
        <Field label={t('spell_slot')}>
          {({ id }) => (
            <Select
              id={id}
              value={String(slot)}
              disabled={busy}
              options={Object.entries(actor.casting?.slots ?? {})
                .filter(([level]) => Number(level) >= rule.level)
                .map(([level, stock]) => ({
                  value: level,
                  label: t('spell_slot_stock', {
                    level,
                    remaining: stock.remaining,
                    maximum: stock.maximum,
                  }),
                  disabled: stock.remaining === 0,
                }))}
              onChange={(value) => setSlot(Number(value))}
            />
          )}
        </Field>
      )}
      {rule && (
        <p>
          {t(rule.bonus_action ? 'bonus_action' : 'action')} ·{' '}
          {t('spell_range', { range: rule.range_ft })}
        </p>
      )}
      {error && <p role="alert">{t('weapon_rejected')}</p>}
    </Modal>
  );
}
