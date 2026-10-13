import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Field, Modal } from '../../ui';
import { Select } from '../../ui/Select';
import type { CombatToken } from './model/types';

interface Props {
  initialTargetId?: string | null | undefined;
  disabled?: boolean;
  actor: CombatToken;
  targets: CombatToken[];
  onClose: () => void;
  onAttack: (
    target: string,
    weapon: string,
    mode: 'default' | 'melee' | 'thrown' | 'ranged',
    twoHanded: boolean,
  ) => Promise<boolean>;
}
export function WeaponAttackDialog({
  actor,
  targets,
  onClose,
  onAttack,
  disabled = false,
  initialTargetId,
}: Props) {
  const { t } = useTranslation('combat');
  const loadout = actor.loadout;
  const eligibleTargets = targets.filter(
    (candidate) => candidate.id !== actor.id && candidate.hp > 0,
  );
  const [weapon, setWeapon] = useState(loadout?.default_weapon ?? 'unarmed');
  const [target, setTarget] = useState(
    eligibleTargets.find((candidate) => candidate.id === initialTargetId)?.id ??
      eligibleTargets[0]?.id ??
      '',
  );
  const [mode, setMode] = useState<'default' | 'melee' | 'thrown' | 'ranged'>('default');
  const [twoHanded, setTwoHanded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const profile = loadout?.attacks.find((attack) => attack.weapon_id === weapon);
  const selectedTarget = targets.find(
    (candidate) => candidate.id === target && candidate.id !== actor.id && candidate.hp > 0,
  );
  const reason = disabled
    ? 'action_turn_unavailable'
    : !selectedTarget
      ? 'action_target_unavailable'
      : !profile || (weapon !== 'unarmed' && (loadout?.quantities[weapon] ?? 0) <= 0)
        ? 'action_equipment_unavailable'
        : profile.ammunition && (loadout?.arrows ?? 0) <= 0
          ? 'action_ammunition_unavailable'
          : loadout?.shield && (profile.two_handed || twoHanded)
            ? 'action_shield_conflict'
            : null;
  const available = reason === null;
  const attack = async () => {
    if (!profile || !target || busy || !available) return;
    setBusy(true);
    setError(false);
    try {
      if (await onAttack(target, weapon, mode, twoHanded)) onClose();
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
      onClose={onClose}
      disableClose={busy}
      title={t('weapon_attack_title')}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            {t('weapon_cancel')}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !target || !available}
            onClick={() => void attack()}
          >
            {t('weapon_confirm')}
          </Button>
        </>
      }
    >
      <Field label={t('weapon_target')}>
        {({ id }) => (
          <Select
            id={id}
            value={target}
            disabled={busy}
            options={eligibleTargets.map((target) => ({
              value: target.id,
              label: `${target.displayNameKey ? t(target.displayNameKey, { defaultValue: target.name }) : target.name} (${target.hp}/${target.maxHp})`,
            }))}
            onChange={setTarget}
          />
        )}
      </Field>
      {reason && <p role="status">{t(reason)}</p>}
      <Field label={t('weapon_equipment')}>
        {({ id }) => (
          <Select
            id={id}
            value={weapon}
            disabled={busy}
            options={(loadout?.attacks ?? []).map((attack) => ({
              value: attack.weapon_id,
              label: t(`weapon_names.${attack.weapon_id}`),
              disabled:
                attack.weapon_id !== 'unarmed' &&
                (loadout?.quantities[attack.weapon_id] ?? 0) === 0,
            }))}
            onChange={(value) => {
              setWeapon(value);
              setMode('default');
              setTwoHanded(false);
            }}
          />
        )}
      </Field>
      {profile?.thrown && (
        <Field label={t('weapon_mode')}>
          {({ id }) => (
            <Select
              id={id}
              value={mode}
              disabled={busy}
              options={[
                { value: 'default', label: t('weapon_mode_default') },
                { value: 'thrown', label: t('weapon_mode_thrown') },
              ]}
              onChange={(value) => {
                setMode(value);
                if (value === 'thrown') setTwoHanded(false);
              }}
            />
          )}
        </Field>
      )}
      {profile?.versatile_damage && mode !== 'thrown' && (
        <Field label={t('weapon_two_handed')}>
          {({ id }) => (
            <Select
              id={id}
              value={twoHanded ? 'yes' : 'no'}
              disabled={busy || Boolean(loadout?.shield)}
              options={[
                { value: 'no', label: t('weapon_one_hand') },
                { value: 'yes', label: t('weapon_two_handed') },
              ]}
              onChange={(value) => setTwoHanded(value === 'yes')}
            />
          )}
        </Field>
      )}
      {profile && (
        <p>
          {t('weapon_bonus', { bonus: profile.attack_bonus })} ·{' '}
          {t('weapon_range', {
            normal:
              mode === 'thrown' || profile.ranged ? profile.normal_range_ft : profile.reach_ft,
            long: mode === 'thrown' || profile.ranged ? profile.long_range_ft : profile.reach_ft,
          })}
        </p>
      )}
      {profile?.ammunition && <p>{t('weapon_arrows', { count: loadout?.arrows ?? 0 })}</p>}
      {error && <p role="alert">{t('weapon_rejected')}</p>}
    </Modal>
  );
}
