import { useTranslation } from 'react-i18next';
import type { MergedEntry } from '../../../state/local_llm/manifest';

export interface ActiveModelPickerProps {
  installedModels: MergedEntry[];
  activeId: string | null;
  onChange: (id: string) => void;
  disabled: boolean;
}

export function ActiveModelPicker({
  installedModels,
  activeId,
  onChange,
  disabled,
}: ActiveModelPickerProps) {
  const { t } = useTranslation('local_llm');

  if (installedModels.length === 0) {
    return <p data-testid="active-picker-empty">{t('no_models_installed')}</p>;
  }

  return (
    <fieldset disabled={disabled}>
      <legend className="sr-only">{t('choose_active_model')}</legend>
      {installedModels.map((m) => (
        <label key={m.id} style={{ display: 'block', padding: '4px 0' }}>
          <input
            type="radio"
            name="active-local-model"
            value={m.id}
            checked={activeId === m.id}
            disabled={disabled}
            onChange={() => onChange(m.id)}
          />{' '}
          {m.display_name} ({m.size_gb} GB)
        </label>
      ))}
    </fieldset>
  );
}
