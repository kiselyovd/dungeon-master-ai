import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CampaignSummary } from '../../api/contracts/campaign';
import {
  canRegisterInvitationProtocol,
  type InvitationPreview,
  parseInvitation,
  registerInvitationProtocol,
} from '../../api/invitations';
import type { GuestRecoveryChoice } from '../../api/rooms';
import { Button } from '../../ui/Button';
import { Field } from '../../ui/Field';
import { Modal } from '../../ui/Modal';
import styles from './Room.module.css';

interface Props {
  initialInvitation?: InvitationPreview | null;
  campaignsLoading?: boolean | undefined;
  campaignsError?: boolean | undefined;
  onRefreshCampaigns?: (() => void) | undefined;
  onRecoverHost?: () => void;
  onPostpone?: () => void;
  saved?: GuestRecoveryChoice[];
  onSaved?: (choice: GuestRecoveryChoice) => void;
  onRecover?: () => void;
  campaigns: CampaignSummary[];
  busy: boolean;
  error: boolean;
  onBack: () => void;
  onHost: (campaign: CampaignSummary, bind: string) => void;
  onJoin: (address: string, room: string, name: string, invitation: string) => void;
}
export function RoomHome({
  initialInvitation,
  campaignsLoading,
  campaignsError,
  onRefreshCampaigns,
  campaigns,
  busy,
  error,
  onBack,
  onHost,
  onJoin,
  onRecover,
  onRecoverHost,
  onPostpone,
  saved,
  onSaved,
}: Props) {
  const { t } = useTranslation('rooms');
  const [postponeOpen, setPostponeOpen] = useState(false);
  const [campaignId, setCampaignId] = useState(campaigns[0]?.id ?? '');
  const [bind, setBind] = useState('0.0.0.0:4123');
  const [address, setAddress] = useState('');
  const [room, setRoom] = useState('');
  const [name, setName] = useState('');
  const [invitation, setInvitation] = useState('');
  const [link, setLink] = useState('');
  const [parsed, setParsed] = useState(initialInvitation ?? null);
  const [manual, setManual] = useState(false);
  const [parsing, setParsing] = useState(false);
  const [linkError, setLinkError] = useState(false);
  const [protocolReady, setProtocolReady] = useState(false);
  const parseGeneration = useRef(0);
  useEffect(() => {
    if (initialInvitation) {
      parseGeneration.current++;
      setParsing(false);
      setParsed(initialInvitation);
      setLink('');
      setManual(false);
      setLinkError(false);
    }
  }, [initialInvitation]);
  useEffect(
    () => () => {
      parseGeneration.current++;
    },
    [],
  );
  const checkLink = async () => {
    if (busy || parsing) return;
    setParsing(true);
    const expected = ++parseGeneration.current;
    setLinkError(false);
    try {
      const value = await parseInvitation(link);
      if (parseGeneration.current !== expected) return;
      setParsed(value);
      setLink('');
      setManual(false);
    } catch {
      if (parseGeneration.current !== expected) return;
      setParsed(null);
      setLinkError(true);
    } finally {
      if (parseGeneration.current === expected) setParsing(false);
    }
  };
  const selection = campaignId || campaigns[0]?.id || '';
  const campaign = campaigns.find((item) => item.id === selection);
  return (
    <main className={styles.root}>
      <div className={styles.content}>
        <header>
          <h1>{t('title')}</h1>
          <p>{t('human')}</p>
          {canRegisterInvitationProtocol() && (
            <Button
              disabled={busy || parsing || protocolReady}
              onClick={() => {
                setParsing(true);
                setLinkError(false);
                void registerInvitationProtocol()
                  .then(() => setProtocolReady(true))
                  .catch(() => setLinkError(true))
                  .finally(() => setParsing(false));
              }}
            >
              {t(protocolReady ? 'link_protocol_ready' : 'link_protocol_enable')}
            </Button>
          )}
          {onRecover && (
            <Button disabled={busy} onClick={onRecover}>
              {t('recover_connection')}
            </Button>
          )}
          <Button onClick={onBack} disabled={busy}>
            {t('back')}
          </Button>
        </header>
        {error && <p role="alert">{t('error')}</p>}
        {linkError && <p role="alert">{t('link_error')}</p>}
        {onPostpone && (
          <Button disabled={busy} onClick={() => setPostponeOpen(true)}>
            {t('postpone')}
          </Button>
        )}
        {saved && saved.length > 0 && (
          <section className={styles.card}>
            <h2>{t('saved_connections')}</h2>
            {saved.map((choice) => (
              <Button key={choice.join_id} disabled={busy} onClick={() => onSaved?.(choice)}>
                {choice.display_name} · {choice.address}
              </Button>
            ))}
          </section>
        )}
        <Modal
          open={postponeOpen}
          title={t('postpone')}
          onClose={() => setPostponeOpen(false)}
          footer={
            <Button
              onClick={() => {
                setPostponeOpen(false);
                onPostpone?.();
              }}
            >
              {t('postpone_confirm')}
            </Button>
          }
        >
          <p>{t('postpone_hint')}</p>
        </Modal>

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
            {campaignsLoading && <p role="status">{t('campaigns_loading')}</p>}
            {campaignsError && <p role="alert">{t('campaigns_unavailable')}</p>}
            {onRefreshCampaigns && (campaignsError || campaigns.length === 0) && (
              <Button disabled={campaignsLoading} onClick={onRefreshCampaigns}>
                {t('campaigns_load')}
              </Button>
            )}
            {onRecoverHost && (
              <Button disabled={busy} onClick={onRecoverHost}>
                {t('recover_host')}
              </Button>
            )}
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
            <details>
              <summary>{t('advanced')}</summary>
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
            </details>
            <Button
              type="submit"
              variant="primary"
              disabled={busy || campaignsLoading || !campaign}
            >
              {busy ? t('connecting') : t('host')}
            </Button>
          </form>
          <form
            className={styles.card}
            onSubmit={(event) => {
              event.preventDefault();
              if (parsed) onJoin(parsed.address, parsed.room_id, name.trim(), parsed.invitation);
              else if (manual) onJoin(address.trim(), room.trim(), name.trim(), invitation.trim());
            }}
          >
            <h2>{t('join')}</h2>
            <p className={styles.meta}>{t('join_hint')}</p>
            <Field label={t('link')}>
              {(props) => (
                <input
                  {...props}
                  className={styles.input}
                  value={link}
                  autoComplete="off"
                  spellCheck={false}
                  data-room-invitation
                  disabled={busy || parsing}
                  onChange={(event) => {
                    setLink(event.target.value);
                    setParsed(null);
                    setLinkError(false);
                  }}
                />
              )}
            </Field>
            <Button disabled={busy || parsing || !link} onClick={() => void checkLink()}>
              {t('link_check')}
            </Button>
            {parsed && (
              <section aria-label={t('link_preview')}>
                <p>
                  {t('address')}: {parsed.address}
                </p>
                <p>
                  {t('room_id')}: {parsed.room_id}
                </p>
                <p>{t('link_confirm_hint')}</p>
              </section>
            )}
            <Field label={t('name')}>
              {(props) => (
                <input
                  {...props}
                  className={styles.input}
                  value={name}
                  disabled={busy || parsing}
                  onChange={(event) => setName(event.target.value)}
                  required
                />
              )}
            </Field>
            <details
              open={manual}
              onToggle={(event) => {
                const open = event.currentTarget.open;
                setManual(open);
                if (open) setParsed(null);
              }}
            >
              <summary>{t('manual_join')}</summary>
              {[
                [t('address'), address, setAddress],
                [t('room_id'), room, setRoom],
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
                      required={manual && !parsed}
                    />
                  )}
                </Field>
              ))}
            </details>
            <Button
              type="submit"
              variant="primary"
              disabled={busy || parsing || (!parsed && !manual)}
            >
              {busy ? t('connecting') : t('join')}
            </Button>
          </form>
        </div>
      </div>
    </main>
  );
}
