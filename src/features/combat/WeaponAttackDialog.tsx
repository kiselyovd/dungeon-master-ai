import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Field, Modal } from '../../ui';
import { Select } from '../../ui/Select';
import type { CombatToken } from './model/types';

interface Props {
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
export function WeaponAttackDialog({ actor, targets, onClose, onAttack }: Props) {
  const { t } = useTranslation('combat');
  const loadout = actor.loadout;
  const [weapon, setWeapon] = useState(loadout?.default_weapon ?? 'unarmed');
  const [target, setTarget] = useState(targets[0]?.id ?? '');
  const [mode, setMode] = useState<'default' | 'melee' | 'thrown' | 'ranged'>('default');
  const [twoHanded, setTwoHanded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const profile = loadout?.attacks.find((attack) => attack.weapon_id === weapon);
  const available =
    profile &&
    (weapon === 'unarmed' || (loadout?.quantities[weapon] ?? 0) > 0) &&
    (!profile.ammunition || (loadout?.arrows ?? 0) > 0) &&
    !(loadout?.shield && (profile.two_handed || twoHanded));
  const attack = async () => {
    if (!profile || !target || busy || !available) return;
    setBusy(true);
    setError(false);
    const accepted = await onAttack(target, weapon, mode, twoHanded);
    setBusy(false);
    if (accepted) onClose();
    else setError(true);
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
            options={targets.map((target) => ({
              value: target.id,
              label: `${target.name} (${target.hp}/${target.maxHp})`,
            }))}
            onChange={setTarget}
          />
        )}
      </Field>
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
      {!available && <p role="status">{t('weapon_unavailable')}</p>}
      {error && <p role="alert">{t('weapon_rejected')}</p>}
    </Modal>
  );
}
