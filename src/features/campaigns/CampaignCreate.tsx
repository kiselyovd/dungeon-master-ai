import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CreateCampaignInput } from '../../api/contracts/campaign';
import {
  type CampaignCreationDraft,
  readCampaignCreationDraft,
  saveCampaignCreationDraft,
} from '../../state/campaignPreparation';
import { Button } from '../../ui/Button';
import { Field } from '../../ui/Field';
import { Icons } from '../../ui/Icons';
import styles from './CampaignCreate.module.css';
import { CampaignPathCard } from './components/CampaignPathCard';

interface CampaignCreateProps {
  busy: boolean;
  onBack: () => void;
  onCreate: (input: CreateCampaignInput) => Promise<void>;
}
export function CampaignCreate({ busy, onBack, onCreate }: CampaignCreateProps) {
  const { t } = useTranslation('campaigns');
  const { t: ta } = useTranslation('adventure');
  const [draft, setDraft] = useState<CampaignCreationDraft>({
    source: 'bundled',
    title: '',
    premise: '',
  });
  const { source, title, premise } = draft;
  const [ready, setReady] = useState(false);
  const [error, setError] = useState(false);
  useEffect(() => {
    let current = true;
    void readCampaignCreationDraft()
      .then((saved) => {
        if (!current) return;
        if (saved) setDraft(saved);
        setReady(true);
      })
      .catch(() => {
        if (current) setError(true);
      });
    return () => {
      current = false;
    };
  }, []);
  function edit(patch: Partial<CampaignCreationDraft>) {
    const next = { ...draft, ...patch };
    setDraft(next);
    void saveCampaignCreationDraft(next).catch(() => setError(true));
  }
  async function create() {
    setError(false);
    try {
      await saveCampaignCreationDraft(draft);
      await onCreate(
        source === 'bundled'
          ? {
              title: ta('title'),
              premise: ta('premise'),
              seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
            }
          : {
              title: title.trim(),
              premise: premise.trim(),
              seed: { kind: 'collaborative', answers: { genre: ['classic_fantasy'] } },
            },
      );
    } catch {
      setError(true);
    }
  }
  return (
    <main className={styles.root} data-art-direction="living-tabletop">
      <div className={styles.sheet}>
        <header className={styles.header}>
          <h1>{t('create_title')}</h1>
        </header>
        <fieldset className={styles.stage} disabled={!ready || busy}>
          <div className={styles.paths} role="radiogroup" aria-label={t('source_title')}>
            <CampaignPathCard
              checked={source === 'bundled'}
              title={t('source_bundled')}
              description={t('source_bundled_hint')}
              icon={<Icons.BookOpen size={20} />}
              onSelect={() => edit({ source: 'bundled' })}
            />
            <CampaignPathCard
              checked={source === 'collaborative'}
              title={t('source_collaborative')}
              description={t('sandbox_hint')}
              icon={<Icons.Sparkle size={20} />}
              onSelect={() => edit({ source: 'collaborative' })}
            />
          </div>
          {source === 'bundled' ? (
            <div className={styles.handoff}>
              <h2>{ta('title')}</h2>
              <p>{ta('premise')}</p>
            </div>
          ) : (
            <>
              <Field label={t('campaign_title')}>
                {(props) => (
                  <input
                    {...props}
                    value={title}
                    maxLength={16000}
                    onChange={(event) => edit({ title: event.target.value })}
                  />
                )}
              </Field>
              <Field label={t('opening_scene')}>
                {(props) => (
                  <textarea
                    {...props}
                    rows={4}
                    value={premise}
                    maxLength={16000}
                    onChange={(event) => edit({ premise: event.target.value })}
                  />
                )}
              </Field>
            </>
          )}
          {error && <p role="alert">{t('preparation_failed')}</p>}
        </fieldset>
        <footer className={styles.actions}>
          <Button onClick={onBack} disabled={busy}>
            {t('back')}
          </Button>
          <Button
            variant="primary"
            onClick={() => void create()}
            disabled={
              !ready || busy || (source === 'collaborative' && (!title.trim() || !premise.trim()))
            }
          >
            {busy ? t('creating') : t('choose_hero')}
          </Button>
        </footer>
      </div>
    </main>
  );
}
