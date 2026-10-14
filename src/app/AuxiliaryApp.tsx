import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  auxiliaryKind,
  closeAuxiliaryWindow,
  focusMainWindow,
  readAuxiliaryProjection,
} from '../api/auxiliaryWindows';
import { CharacterSheetView } from '../components/CharacterSheetView';
import { JournalViewer } from '../components/JournalViewer';
import { Button } from '../ui/Button';
import styles from './AuxiliaryApp.module.css';
import { type AuxiliaryProjection, parseAuxiliaryProjection } from './auxiliaryProjection';

export function AuxiliaryApp() {
  const { t, i18n } = useTranslation('common');
  const kind = auxiliaryKind();
  const [projection, setProjection] = useState<AuxiliaryProjection | null>(null);
  useEffect(() => {
    let active = true;
    let afterSequence: number | null = null;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      try {
        const response = await readAuxiliaryProjection(afterSequence);
        if (
          response &&
          typeof response === 'object' &&
          'unchangedSequence' in response &&
          response.unchangedSequence === afterSequence
        )
          return;
        const value = parseAuxiliaryProjection(response);
        if (!active) return;
        afterSequence = value?.sequence ?? null;
        setProjection((previous) => (previous?.sequence === value?.sequence ? previous : value));
        if (value && i18n.language !== value.language) await i18n.changeLanguage(value.language);
      } catch {
        if (active) {
          afterSequence = null;
          setProjection(null);
        }
      } finally {
        if (active) timer = setTimeout(() => void poll(), 1000);
      }
    };
    void poll();
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [i18n]);
  const close = () => void closeAuxiliaryWindow();
  return (
    <main className={styles.root}>
      <header className={styles.header}>
        <strong>Dungeon Master AI</strong>
        <span>{t(kind === 'hero' ? 'hero' : 'journal')}</span>
        <span>{t('aux_readonly')}</span>
        <Button onClick={() => void focusMainWindow()}>{t('aux_return')}</Button>
        <Button onClick={close}>{t('close')}</Button>
      </header>
      <div className={styles.content}>
        {!projection?.ready || !kind ? (
          <p role="status">{t('context_unavailable')}</p>
        ) : kind === 'hero' ? (
          projection.hero ? (
            <CharacterSheetView embedded open pc={projection.hero} onClose={close} />
          ) : (
            <p>{t('aux_no_hero')}</p>
          )
        ) : (
          <JournalViewer embedded entries={projection.journal} onClose={close} />
        )}
      </div>
    </main>
  );
}
