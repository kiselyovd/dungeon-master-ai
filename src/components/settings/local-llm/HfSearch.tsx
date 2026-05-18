import { useTranslation } from 'react-i18next';

/**
 * Placeholder body for the "Search Hugging Face" collapsible card.
 *
 * Replaced in M9-DM Tasks 15-19 by the real search UI: token entry, filter
 * panel, result cards, and license-accept flow. The placeholder exists so the
 * Task 14 ModelSelector container can ship the three-section shell without
 * waiting on the HF backend.
 */
export function HfSearch() {
  const { t } = useTranslation('local_llm');
  return <p data-testid="hf-search-placeholder">{t('hf_coming_soon')}</p>;
}
