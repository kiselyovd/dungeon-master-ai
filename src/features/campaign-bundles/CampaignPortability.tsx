import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  type BundleImportChoice,
  type BundleImportResult,
  type BundleKind,
  bundleFailureCode,
  cancelCampaignImport,
  confirmCampaignImport,
  exportCampaignFile,
  nativeBundlesAvailable,
  pickCampaignImport,
  revealCampaignFile,
} from '../../api/campaignBundles';
import { Button } from '../../ui/Button';
import { Field } from '../../ui/Field';
import { Modal } from '../../ui/Modal';
import styles from './CampaignPortability.module.css';

interface Props {
  scopeId: string;
  campaigns: { id: string; title: string }[];
  guest?: boolean;
  canImport?: boolean;
  onImported?: (result: BundleImportResult) => Promise<void> | void;
  onOpen?: (campaign: string) => Promise<void> | void;
}
export function CampaignPortability({
  scopeId,
  campaigns,
  guest = false,
  canImport = true,
  onImported,
  onOpen,
}: Props) {
  const { t, i18n } = useTranslation('common');
  const available = nativeBundlesAvailable();
  const [open, setOpen] = useState(false);
  const [campaignId, setCampaignId] = useState(campaigns[0]?.id ?? '');
  const [kind, setKind] = useState<BundleKind>(guest ? 'player_dossier' : 'full_campaign_backup');
  const [busy, setBusy] = useState<'import' | 'export' | 'pick' | null>(null);
  const [choice, setChoice] = useState<BundleImportChoice | null>(null);
  const [imported, setImported] = useState<BundleImportResult | null>(null);
  const [saved, setSaved] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const current = useRef(0);
  const pending = useRef<BundleImportChoice | null>(null);
  const campaignList = useRef(campaigns);
  campaignList.current = campaigns;
  const activeScope = useRef(scopeId);
  const firstCampaignId = campaigns[0]?.id ?? '';
  const selectedCampaignExists = campaigns.some((campaign) => campaign.id === campaignId);
  useEffect(() => {
    if (!selectedCampaignExists && !busy) setCampaignId(firstCampaignId);
  }, [selectedCampaignExists, firstCampaignId, busy]);
  pending.current = choice;
  useEffect(() => {
    activeScope.current = scopeId;
    current.current++;
    setCampaignId(campaignList.current[0]?.id ?? '');
    setKind(guest ? 'player_dossier' : 'full_campaign_backup');
    setOpen(false);
    setChoice(null);
    setImported(null);
    setFailure(null);
    setBusy(null);
    return () => {
      current.current++;
      const previous = pending.current;
      if (previous) void cancelCampaignImport(previous.operationId).catch(() => {});
    };
  }, [scopeId, guest]);
  const language = () => (i18n.language.startsWith('ru') ? ('ru' as const) : ('en' as const));
  const error = (cause: unknown) => setFailure(bundleFailureCode(cause));
  const close = () => {
    if (busy) return;
    if (choice) void cancelCampaignImport(choice.operationId).catch(error);
    setChoice(null);
    setImported(null);
    setOpen(false);
    setFailure(null);
  };
  const pick = async () => {
    if (busy || !available) return;
    const expected = current.current;
    setBusy('pick');
    setSaved(null);
    setFailure(null);
    setImported(null);
    try {
      const result = await pickCampaignImport(language());
      if (expected !== current.current) {
        if (result) await cancelCampaignImport(result.operationId);
        return;
      }
      setChoice(result);
      if (result) setOpen(true);
    } catch (cause) {
      if (expected === current.current) error(cause);
    } finally {
      if (expected === current.current) setBusy(null);
    }
  };
  const commit = async () => {
    if (!choice || busy) return;
    const expected = current.current;
    setBusy('import');
    setFailure(null);
    try {
      const result = await confirmCampaignImport(choice.operationId);
      if (expected !== current.current) return;
      setChoice(null);
      setImported(result);
      await onImported?.(result);
    } catch (cause) {
      if (expected === current.current) error(cause);
    } finally {
      if (expected === current.current) setBusy(null);
    }
  };
  const exportFile = async () => {
    if (!campaignId || busy || !available || (guest && kind !== 'player_dossier')) return;
    const expected = current.current;
    setBusy('export');
    setFailure(null);
    setSaved(null);
    try {
      const result = await exportCampaignFile(campaignId, kind, language());
      if (expected === current.current) setSaved(result?.fileName ?? null);
    } catch (cause) {
      if (expected === current.current) error(cause);
    } finally {
      if (expected === current.current) setBusy(null);
    }
  };
  const failureText = failure
    ? t(
        failure === 'bundle_not_restorable'
          ? 'bundle_dossier_not_backup'
          : failure === 'native_bundle_target_exists'
            ? 'bundle_target_exists'
            : failure === 'native_bundle_delivery_unknown'
              ? 'bundle_retry_safe'
              : failure === 'bundle_export_forbidden'
                ? 'bundle_no_actor'
                : 'bundle_failed',
      )
    : null;
  return (
    <>
      <section className={styles.actions} aria-label={t('bundle_title')}>
        {canImport && (
          <Button disabled={!available || Boolean(busy)} onClick={() => void pick()}>
            {t('bundle_import')}
          </Button>
        )}
        <Button
          disabled={!available || !campaigns.length || Boolean(busy)}
          onClick={() => {
            setOpen(true);
            setChoice(null);
            setImported(null);
            setFailure(null);
          }}
        >
          {t('bundle_export')}
        </Button>
        {!available && <p>{t('bundle_desktop_only')}</p>}
        {busy && !open && <p role="status">{t('bundle_working')}</p>}
        {failureText && !open && <p role="alert">{failureText}</p>}
      </section>
      <Modal
        open={open}
        onClose={close}
        disableClose={Boolean(busy)}
        title={t(choice || imported ? 'bundle_import' : 'bundle_export')}
        footer={
          <>
            <Button disabled={Boolean(busy)} onClick={close}>
              {t(!choice && (imported || saved) ? 'close' : 'cancel')}
            </Button>
            {choice && (
              <Button variant="primary" disabled={Boolean(busy)} onClick={() => void commit()}>
                {t('bundle_confirm_import')}
              </Button>
            )}
            {!choice && !imported && (
              <Button
                variant="primary"
                disabled={Boolean(busy) || !campaignId}
                onClick={() => void exportFile()}
              >
                {t('bundle_save_file')}
              </Button>
            )}
            {imported && onOpen && (
              <Button
                variant="primary"
                disabled={Boolean(busy)}
                onClick={() =>
                  void Promise.resolve(onOpen(imported.campaign_id)).then(close).catch(error)
                }
              >
                {t('bundle_open_imported')}
              </Button>
            )}
          </>
        }
      >
        {choice ? (
          <>
            <p>{choice.fileName}</p>
            <strong>{choice.preview.title}</strong>
            <p>
              {t('bundle_summary', {
                characters: choice.preview.character_count,
                messages: choice.preview.message_count,
                media: choice.preview.media_count,
              })}
            </p>
            <p>{t('bundle_new_campaign')}</p>
            {choice.preview.private_backup && <p>{t('bundle_private_backup')}</p>}
          </>
        ) : imported ? (
          <p role="status">{t('bundle_import_done')}</p>
        ) : (
          <>
            <Field label={t('bundle_campaign')}>
              {(props) => (
                <select
                  {...props}
                  disabled={Boolean(busy)}
                  value={campaignId}
                  onChange={(event) => setCampaignId(event.target.value)}
                >
                  {campaigns.map((campaign) => (
                    <option key={campaign.id} value={campaign.id}>
                      {campaign.title}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Field label={t('bundle_format')}>
              {(props) => (
                <select
                  {...props}
                  disabled={Boolean(busy) || guest}
                  value={kind}
                  onChange={(event) => setKind(event.target.value as BundleKind)}
                >
                  <option value="full_campaign_backup" disabled={guest}>
                    {t('bundle_backup')}
                  </option>
                  <option value="player_dossier">{t('bundle_dossier')}</option>
                </select>
              )}
            </Field>
            <p>
              {t(kind === 'full_campaign_backup' ? 'bundle_private_backup' : 'bundle_dossier_hint')}
            </p>
            {saved && (
              <>
                <p role="status">{t('bundle_export_done', { name: saved })}</p>
                <Button onClick={() => void revealCampaignFile().catch(error)}>
                  {t('bundle_reveal')}
                </Button>
              </>
            )}
          </>
        )}
        {busy && (
          <p role="status">
            {t(
              busy === 'pick'
                ? 'bundle_validating'
                : busy === 'import'
                  ? 'bundle_importing'
                  : 'bundle_exporting',
            )}
          </p>
        )}
        {failureText && <p role="alert">{failureText}</p>}
      </Modal>
    </>
  );
}
