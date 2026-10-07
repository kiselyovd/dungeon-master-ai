import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CampaignSummary } from '../../api/contracts/campaign';
import { Button } from '../../ui/Button';
import { Field } from '../../ui/Field';
import styles from './Room.module.css';

interface Props {
  campaigns: CampaignSummary[];
  busy: boolean;
  error: boolean;
  onBack: () => void;
  onHost: (campaign: CampaignSummary, bind: string) => void;
  onJoin: (address: string, room: string, name: string, invitation: string) => void;
}
export function RoomHome({ campaigns, busy, error, onBack, onHost, onJoin }: Props) {
  const { t } = useTranslation('rooms');
  const [campaignId, setCampaignId] = useState(campaigns[0]?.id ?? '');
  const [bind, setBind] = useState('0.0.0.0:4123');
  const [address, setAddress] = useState('');
  const [room, setRoom] = useState('');
  const [name, setName] = useState('');
  const [invitation, setInvitation] = useState('');
  const selection = campaignId || campaigns[0]?.id || '';
  const campaign = campaigns.find((item) => item.id === selection);
  return (
    <main className={styles.root}>
      <div className={styles.content}>
        <header>
          <h1>{t('title')}</h1>
          <p>{t('human')}</p>
          <Button onClick={onBack} disabled={busy}>
            {t('back')}
          </Button>
        </header>
        {error && <p role="alert">{t('error')}</p>}
        <div className={styles.columns}>
          <form
            className={styles.card}
            onSubmit={(event) => {
              event.preventDefault();
              if (campaign) onHost(campaign, bind.trim());
            }}
          >
            <h2>{t('host')}</h2>
            <p className={styles.meta}>{t('host_hint')}</p>
            <Field label={t('campaign')}>
              {(props) => (
                <select
                  {...props}
                  className={styles.input}
                  value={selection}
                  disabled={busy}
                  onChange={(event) => setCampaignId(event.target.value)}
                >
                  {campaigns.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.title}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Field label={t('bind')}>
              {(props) => (
                <input
                  {...props}
                  className={styles.input}
                  value={bind}
                  disabled={busy}
                  onChange={(event) => setBind(event.target.value)}
                  required
                />
              )}
            </Field>
            <Button type="submit" variant="primary" disabled={busy || !campaign}>
              {busy ? t('connecting') : t('host')}
            </Button>
          </form>
          <form
            className={styles.card}
            onSubmit={(event) => {
              event.preventDefault();
              onJoin(address.trim(), room.trim(), name.trim(), invitation.trim());
            }}
          >
            <h2>{t('join')}</h2>
            <p className={styles.meta}>{t('join_hint')}</p>
            {[
              [t('address'), address, setAddress],
              [t('room_id'), room, setRoom],
              [t('name'), name, setName],
              [t('invitation'), invitation, setInvitation],
            ].map(([label, value, setter]) => (
              <Field key={String(label)} label={String(label)}>
                {(props) => (
                  <input
                    {...props}
                    className={styles.input}
                    value={String(value)}
                    disabled={busy}
                    onChange={(event) => (setter as (value: string) => void)(event.target.value)}
                    required
                  />
                )}
              </Field>
            ))}
            <Button type="submit" variant="primary" disabled={busy}>
              {busy ? t('connecting') : t('join')}
            </Button>
          </form>
        </div>
      </div>
    </main>
  );
}
