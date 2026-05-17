import { useTranslation } from 'react-i18next';
import { Icons } from '../ui/Icons';
import styles from './StatusBar.module.css';

export type ProviderStatus = 'connected' | 'error' | 'loading';
export type SettingsTab = 'chat' | 'image' | 'video';

export interface ModalityIndicator {
  enabled: boolean;
  /** Short label rendered in the pill, e.g. "balanced", "live", "off". */
  label: string;
}

interface Props {
  provider: string;
  model: string;
  status?: ProviderStatus;
  savedAgo?: { minutes: number } | 'now' | null;
  image: ModalityIndicator;
  video: ModalityIndicator;
  onOpenSettings?: (tab: SettingsTab) => void;
}

export function StatusBar({
  provider,
  model,
  status = 'connected',
  savedAgo = null,
  image,
  video,
  onOpenSettings,
}: Props) {
  const { t } = useTranslation('common');

  const dotClass =
    status === 'connected'
      ? styles.dotConnected
      : status === 'error'
        ? styles.dotError
        : styles.dotLoading;

  const savedLabel =
    savedAgo === null
      ? null
      : savedAgo === 'now'
        ? t('saved_now')
        : t('saved_min_ago', { count: savedAgo.minutes });

  const openIfPossible = (tab: SettingsTab) => () => {
    if (onOpenSettings) onOpenSettings(tab);
  };

  return (
    <footer className={styles.statusBar} role="status" aria-live="polite">
      <button
        type="button"
        className={styles.modalityBtn}
        onClick={openIfPossible('chat')}
        aria-label={t('chat_provider_settings')}
        title={t('chat_provider_settings')}
      >
        <span className={`${styles.dot} ${dotClass}`} />
        <span className={styles.label}>{t('provider')}:</span>
        <span className={styles.value}>{provider}</span>
        <Icons.Cpu size={11} />
        <span className={styles.valueMono}>{model}</span>
      </button>

      <div className={styles.divider} />

      <button
        type="button"
        className={`${styles.modalityBtn} ${image.enabled ? styles.modalityOn : styles.modalityOff}`}
        onClick={openIfPossible('image')}
        aria-label={t('image_gen_settings')}
        title={t('image_gen_settings')}
      >
        <Icons.Image size={11} />
        <span className={styles.label}>{t('image_modality')}:</span>
        <span className={styles.value}>{image.label}</span>
      </button>

      <div className={styles.divider} />

      <button
        type="button"
        className={`${styles.modalityBtn} ${video.enabled ? styles.modalityOn : styles.modalityOff}`}
        onClick={openIfPossible('video')}
        aria-label={t('video_gen_settings')}
        title={t('video_gen_settings')}
      >
        <span className={styles.label}>{t('video_modality')}:</span>
        <span className={styles.value}>{video.label}</span>
      </button>

      <div className={styles.spacer} />

      {savedLabel !== null && (
        <div className={styles.item}>
          <Icons.Save size={11} />
          <span className={styles.label}>{t('saved')}:</span>
          <span className={styles.value}>{savedLabel}</span>
        </div>
      )}
    </footer>
  );
}
