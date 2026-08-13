import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CampaignSeed, CreateCampaignInput } from '../../api/contracts/campaign';
import { Icons } from '../../ui/Icons';
import styles from './CampaignCreate.module.css';
import { CampaignPathCard } from './components/CampaignPathCard';

type SourceKind = CampaignSeed['kind'];

interface CampaignCreateProps {
  busy: boolean;
  onBack: () => void;
  onCreate: (input: CreateCampaignInput) => Promise<void>;
}

export function CampaignCreate({ busy, onBack, onCreate }: CampaignCreateProps) {
  const { t } = useTranslation('campaigns');
  const [step, setStep] = useState(1);
  const [source, setSource] = useState<SourceKind>('collaborative');
  const [title, setTitle] = useState('');
  const [premise, setPremise] = useState('');

  const isFutureSource = step === 2 && source !== 'collaborative';
  const canAdvance = step !== 2 || (title.trim().length > 0 && premise.trim().length > 0);

  async function finish() {
    const seed: CampaignSeed = {
      kind: 'collaborative',
      answers: { genre: ['classic_fantasy'] },
    };
    await onCreate({ title: title.trim(), premise: premise.trim(), seed });
  }

  return (
    <main className={styles.root} data-art-direction="living-tabletop">
      <div className={styles.sheet}>
        <header className={styles.header}>
          <div>
            <span className={styles.eyebrow}>{t('create_eyebrow')}</span>
            <h1>{t('create_title')}</h1>
          </div>
          <span className={styles.counter}>{t('step_counter', { current: step, total: 4 })}</span>
        </header>

        <ol className={styles.steps} aria-label={t('creation_steps')}>
          {[1, 2, 3, 4].map((item) => (
            <li key={item} data-active={item === step} data-complete={item < step}>
              <span>{item}</span>
              {t(`step_${item}`)}
            </li>
          ))}
        </ol>

        <section className={styles.stage}>
          {step === 1 && (
            <>
              <h2>{t('source_title')}</h2>
              <p>{t('source_body')}</p>
              <div className={styles.paths} role="radiogroup" aria-label={t('source_title')}>
                <CampaignPathCard
                  checked={source === 'collaborative'}
                  title={t('source_collaborative')}
                  description={t('source_collaborative_hint')}
                  icon={<Icons.Sparkle size={20} />}
                  onSelect={() => setSource('collaborative')}
                />
                <CampaignPathCard
                  checked={source === 'document'}
                  title={t('source_document')}
                  description={t('source_document_hint')}
                  icon={<Icons.Scroll size={20} />}
                  onSelect={() => setSource('document')}
                />
                <CampaignPathCard
                  checked={source === 'bundled'}
                  title={t('source_bundled')}
                  description={t('source_bundled_hint')}
                  icon={<Icons.BookOpen size={20} />}
                  onSelect={() => setSource('bundled')}
                />
              </div>
              {source !== 'collaborative' && (
                <aside className={styles.notice}>{t('source_planned')}</aside>
              )}
            </>
          )}

          {step === 2 && source === 'collaborative' && (
            <>
              <h2>{t('tuning_title')}</h2>
              <p>{t('tuning_body')}</p>
              <label className={styles.field}>
                <span>{t('campaign_title')}</span>
                <input value={title} onChange={(event) => setTitle(event.target.value)} />
              </label>
              <label className={styles.field}>
                <span>{t('campaign_premise')}</span>
                <textarea
                  rows={4}
                  value={premise}
                  onChange={(event) => setPremise(event.target.value)}
                />
              </label>
            </>
          )}

          {isFutureSource && (
            <div className={styles.handoff}>
              {source === 'document' ? <Icons.Scroll size={28} /> : <Icons.BookOpen size={28} />}
              <h2>{source === 'document' ? t('source_document') : t('source_bundled')}</h2>
              <p>{t('source_planned')}</p>
            </div>
          )}

          {step === 3 && (
            <div className={styles.handoff}>
              <Icons.User size={28} />
              <h2>{t('character_handoff_title')}</h2>
              <p>{t('character_handoff_body')}</p>
            </div>
          )}

          {step === 4 && (
            <div className={styles.handoff}>
              <Icons.Map size={28} />
              <h2>{t('preparation_handoff_title')}</h2>
              <p>{t('preparation_handoff_body')}</p>
            </div>
          )}
        </section>

        <footer className={styles.actions}>
          <button type="button" onClick={step === 1 ? onBack : () => setStep(step - 1)}>
            <Icons.ChevronLeft size={14} />
            {t('back')}
          </button>
          {step < 4 && !isFutureSource ? (
            <button
              type="button"
              className={styles.primaryAction}
              disabled={!canAdvance}
              onClick={() => setStep(step + 1)}
            >
              {t('next')}
              <Icons.ChevronRight size={14} />
            </button>
          ) : step === 4 ? (
            <button
              type="button"
              className={styles.primaryAction}
              disabled={busy}
              onClick={() => void finish()}
            >
              <Icons.Sparkle size={15} />
              {busy ? t('creating') : t('create_campaign')}
            </button>
          ) : null}
        </footer>
      </div>
    </main>
  );
}
