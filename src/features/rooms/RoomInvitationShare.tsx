import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { createInvitation } from '../../api/invitations';
import { Button } from '../../ui/Button';
import { Field } from '../../ui/Field';
import styles from './Room.module.css';

interface Props {
  address: string;
  roomId: string;
  invitation: string;
  busy: boolean;
}
export function RoomInvitationShare({ address, roomId, invitation, busy }: Props) {
  const { t } = useTranslation('rooms');
  const [joinAddress, setJoinAddress] = useState(
    address.startsWith('0.0.0.0:') || address.startsWith('[::]:') ? '' : address,
  );
  const [link, setLink] = useState('');
  const [failed, setFailed] = useState(false);
  const [copied, setCopied] = useState(false);
  const [working, setWorking] = useState(false);
  const scope = `${roomId}:${invitation}:${joinAddress}`;
  const latest = useRef(scope);
  latest.current = scope;
  // biome-ignore lint/correctness/useExhaustiveDependencies: replacing an invitation/address must discard the previous share link.
  useEffect(() => {
    setLink('');
    setCopied(false);
    setFailed(false);
  }, [scope]);
  const build = async () => {
    if (busy || working) return;
    const expected = scope;
    setWorking(true);
    setFailed(false);
    try {
      const value = await createInvitation({
        address: joinAddress.trim(),
        room_id: roomId,
        invitation,
      });
      if (expected === latest.current) setLink(value);
    } catch {
      if (expected === latest.current) setFailed(true);
    } finally {
      setWorking(false);
    }
  };
  return (
    <>
      <p className={styles.meta}>{t('share_hint')}</p>
      <Field label={t('share_address')}>
        {(props) => (
          <input
            {...props}
            className={styles.input}
            value={joinAddress}
            disabled={busy || working}
            onChange={(event) => setJoinAddress(event.target.value)}
            placeholder="192.168.1.20:4123"
          />
        )}
      </Field>
      <Button disabled={busy || working || !joinAddress} onClick={() => void build()}>
        {t('link_create')}
      </Button>
      {link && (
        <>
          <Field label={t('link')}>
            {(props) => (
              <input
                {...props}
                className={styles.input}
                readOnly
                value={link}
                data-room-invitation
              />
            )}
          </Field>
          <Button
            disabled={busy || working}
            onClick={() => {
              const expected = scope;
              setFailed(false);
              void navigator.clipboard
                .writeText(link)
                .then(() => {
                  if (latest.current === expected) setCopied(true);
                })
                .catch(() => {
                  if (latest.current === expected) setFailed(true);
                });
            }}
          >
            {t(copied ? 'link_copied' : 'link_copy')}
          </Button>
        </>
      )}
      {failed && <p role="alert">{t('link_error')}</p>}
    </>
  );
}
