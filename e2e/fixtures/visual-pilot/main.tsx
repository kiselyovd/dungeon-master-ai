/** Nonshipping visual fixture. No gameplay, store wiring or authoritative mutations. */
import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { useTranslation } from 'react-i18next';
import { AppShell } from '../../../src/app/AppShell';
import i18n from '../../../src/i18n';
import { Button } from '../../../src/ui/Button';
import { Icons } from '../../../src/ui/Icons';
import '../../../src/styles/globals.css';
import './pilot.css';
import portrait from '../../../src/assets/living-tabletop/v2/hero-fighter-01-v2.webp';
import map from '../../../src/assets/living-tabletop/v2/map-archive-v2.webp';
import oak from '../../../src/assets/living-tabletop/v2/material-oak-v2.webp';
import miniature from '../../../src/assets/living-tabletop/v2/token-fighter-01-v2.webp';
import archive from './archive.json';
import en from './en.json';
import ru from './ru.json';

i18n.addResourceBundle('en', 'visual-pilot', en);
i18n.addResourceBundle('ru', 'visual-pilot', ru);
void i18n.changeLanguage(new URLSearchParams(location.search).get('lang') === 'en' ? 'en' : 'ru');

function Pilot() {
  const { t } = useTranslation('visual-pilot');
  const [coop, setCoop] = useState(false);
  const [grid, setGrid] = useState(true);
  return (
    <AppShell>
      <div className="pilot" style={{ '--pilot-oak': `url(${oak})` } as React.CSSProperties}>
        <header className="pilot-header">
          <Icons.Crest size={26} />
          <div className="pilot-heading">
            <small>{t('campaign')}</small>
            <h1>{t('location')}</h1>
          </div>
          <Button onClick={() => setCoop(!coop)}>{t(coop ? 'coop' : 'solo')}</Button>
          <Button
            variant="ghost"
            onClick={() => void i18n.changeLanguage(i18n.language === 'ru' ? 'en' : 'ru')}
          >
            {i18n.language === 'ru' ? 'EN' : 'РУ'}
          </Button>
          <Button variant="ghost" aria-label={t('settings')}>
            <Icons.Settings size={18} />
          </Button>
        </header>
        <div className="pilot-layout">
          <aside className="pilot-hero pilot-panel">
            <img className="pilot-portrait" src={portrait} alt={t('portraits')} />
            <div className="pilot-identity">
              <h2>{t('hero')}</h2>
              <p>{t('class')}</p>
            </div>
            <div className="pilot-vitals">
              <span>{t('hp')}</span>
              <strong>
                12 <span>/ 12</span>
              </strong>
            </div>
            <div className="pilot-vital-track">
              <span />
            </div>
            <p className="pilot-context">
              <Icons.Shield size={16} />
              {t('intent')}
            </p>
            <div className="pilot-hero-actions">
              <Button>{t('journal')}</Button>
              <Button>{t('npc')}</Button>
            </div>
          </aside>
          <section className="pilot-table">
            <div className="pilot-table-heading">
              <span className="pilot-turn">{t('turn')}</span>
              <span>{t('round')}</span>
              <Button variant="ghost" onClick={() => setGrid(!grid)}>
                {t('grid')}
              </Button>
            </div>
            <div
              className="pilot-map"
              data-grid={grid}
              style={
                {
                  '--pilot-map': `url(${map})`,
                  '--pilot-cols': archive.width,
                  '--pilot-rows': archive.height,
                } as React.CSSProperties
              }
            >
              <div className="pilot-map-grid" />
              {archive.blocked.map((cell) => (
                <div
                  key={cell.y}
                  data-wall
                  className="pilot-wall"
                  style={{
                    left: `${(cell.x / 12) * 100}%`,
                    top: `${(cell.y / 8) * 100}%`,
                    width: `${100 / 12}%`,
                    height: '12.5%',
                  }}
                />
              ))}
              <div className="pilot-exit" style={{ left: '12.5%', top: '43.75%' }}>
                <Icons.Footprints size={16} />
              </div>
              <div className="pilot-token" style={{ left: '29.1667%', top: '43.75%' }}>
                <span className="pilot-token-base" />
                <img src={miniature} alt={t('figure')} />
              </div>
              <div className="pilot-distance">5 ft</div>
            </div>
            <div className="pilot-tray pilot-panel">
              <span>{t('scale')}</span>
              {[32, 48, 64].map((size) => (
                <div key={size} className="pilot-scale">
                  <img src={miniature} alt="" style={{ width: size, height: size }} />
                  <small>{size}px</small>
                </div>
              ))}
              <div className="pilot-material" title={t('materials')} />
            </div>
          </section>
          <aside className="pilot-story pilot-panel">
            <div className="pilot-story-heading">
              <Icons.Crest size={18} />
              <span>{t('dm')}</span>
            </div>
            <div className="pilot-narration">
              <p>{t('message')}</p>
            </div>
            <div className="pilot-chat-composer">
              <textarea aria-label={t('placeholder')} placeholder={t('placeholder')} rows={3} />
              <Button variant="primary">{t('send')}</Button>
            </div>
          </aside>
        </div>
      </div>
    </AppShell>
  );
}
const root = document.getElementById('root');
if (!root) throw new Error('pilot root missing');
createRoot(root).render(<Pilot />);
