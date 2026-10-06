import { useTranslation } from 'react-i18next';
import type { AdventureAction, AdventureProjection } from '../../api/contracts/adventure';
import { Button } from '../../ui/Button';
import { Modal } from '../../ui/Modal';
import styles from './AdventurePanel.module.css';

interface Props {
  open: boolean;
  projection: AdventureProjection;
  pending: boolean;
  error: boolean;
  retryRequired: boolean;
  onAction: (action: AdventureAction) => void;
  onRetry: () => void;
  onClose: () => void;
}

export function AdventurePanel({
  open,
  projection,
  pending,
  error,
  retryRequired,
  onAction,
  onRetry,
  onClose,
}: Props) {
  const { t } = useTranslation('adventure');
  const { state, available_actions: actions } = projection;
  const label = (action: AdventureAction) =>
    action.kind === 'move'
      ? t('actions.move', { location: t(`locations.${action.location}`) })
      : t(`actions.${action.kind}`);
  return (
    <Modal
      open={open}
      title={t('title')}
      onClose={onClose}
      footer={<Button onClick={onClose}>{t('close')}</Button>}
    >
      <div className={styles.content}>
        <section>
          <h2>{t(`locations.${state.location}`)}</h2>
          <p>{t(`scenes.${state.location}`)}</p>
        </section>
        {state.ending && <p role="status">{t(`ending.${state.ending}`)}</p>}
        {pending && <p role="status">{t('pending')}</p>}
        {error && <p role="alert">{t('error')}</p>}
        {retryRequired && (
          <Button disabled={pending} onClick={onRetry}>
            {t('retry')}
          </Button>
        )}
        {actions.length > 0 && (
          <section>
            <h3>{t('actions_title')}</h3>
            <div className={styles.actions}>
              {actions.map((action) => (
                <Button
                  key={JSON.stringify(action)}
                  disabled={pending || retryRequired}
                  onClick={() => onAction(action)}
                >
                  {label(action)}
                </Button>
              ))}
            </div>
          </section>
        )}
        <section>
          <h3>{t('facts_title')}</h3>
          <ul>
            {state.facts.map((fact) => (
              <li key={fact}>{t(`facts.${fact}`)}</li>
            ))}
          </ul>
        </section>
        {state.history.length > 0 && (
          <details>
            <summary>{t('history_title')}</summary>
            <ol>
              {state.history.map((action, index) => (
                <li key={`${index}-${JSON.stringify(action)}`}>{label(action)}</li>
              ))}
            </ol>
          </details>
        )}
      </div>
    </Modal>
  );
}
