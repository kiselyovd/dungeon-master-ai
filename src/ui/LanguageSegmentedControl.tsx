import styles from './LanguageSegmentedControl.module.css';

type LanguageCode = 'en' | 'ru';

interface LanguageSegmentedControlProps {
  value: LanguageCode;
  onChange: (language: LanguageCode) => void;
  ariaLabel: string;
}

const LANGUAGES: readonly { value: LanguageCode; label: string }[] = [
  { value: 'ru', label: 'РУ' },
  { value: 'en', label: 'EN' },
];

export function LanguageSegmentedControl({
  value,
  onChange,
  ariaLabel,
}: LanguageSegmentedControlProps) {
  return (
    <fieldset className={styles.control} aria-label={ariaLabel} data-value={value}>
      <span className={styles.indicator} aria-hidden="true" />
      {LANGUAGES.map((language) => (
        <button
          key={language.value}
          type="button"
          className={styles.option}
          aria-pressed={value === language.value}
          onClick={() => onChange(language.value)}
        >
          {language.label}
        </button>
      ))}
    </fieldset>
  );
}
