import { useTranslation } from 'react-i18next';
import styles from './LicenseRestrictedBanner.module.css';

interface Props {
  modality: 'image' | 'video' | null;
  activePresetName: string | null;
}

/**
 * Inline warning rendered above ImageTab/VideoTab radio lists when the active
 * preset is blocked by licenseRestrictedMode. No auto-switch - user must
 * explicitly pick an OSS preset OR turn off restriction mode.
 */
export function LicenseRestrictedBanner({ modality, activePresetName }: Props) {
  const { t } = useTranslation('settings');
  if (!modality || !activePresetName) return null;
  return (
    <div role="alert" className={styles.banner}>
      {t('license_banner_active_preset_blocked', {
        name: activePresetName,
        modality: t(
          modality === 'image' ? 'license_banner_modality_image' : 'license_banner_modality_video',
        ),
      })}
    </div>
  );
}
