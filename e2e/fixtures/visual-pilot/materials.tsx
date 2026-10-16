/** Nonshipping material/alpha/repetition specimen; uses real production tokens. */
import { createRoot } from 'react-dom/client';
import { VISUAL_ASSET_ENTRIES } from '../../../src/assets/visualCatalog.generated';
import { Button } from '../../../src/ui/Button';
import { Field } from '../../../src/ui/Field';
import '../../../src/styles/globals.css';
import './materials.css';

const materials = VISUAL_ASSET_ENTRIES.filter(
  (asset) => asset.version === 2 && asset.category === 'material',
);
const frames = VISUAL_ASSET_ENTRIES.filter(
  (asset) => asset.version === 2 && asset.category === 'frame',
);
const root = document.getElementById('root');
if (!root) throw new Error('material_specimen_root_missing');
createRoot(root).render(
  <main className="material-specimen">
    <h1>Material / frame QA</h1>
    {materials.map((asset) => (
      <section key={asset.id} data-material={asset.id}>
        <h2>{asset.id}</h2>
        <div className="material-specimen-pair">
          <div
            className="material-specimen-single"
            style={{ backgroundImage: `url(${asset.uri})` }}
          />
          <div
            className="material-specimen-repeated"
            style={{ backgroundImage: `url(${asset.uri})` }}
          />
        </div>
      </section>
    ))}
    {frames.map((asset) => (
      <section key={asset.id} data-frame={asset.id}>
        <h2>{asset.id}</h2>
        <div className="material-specimen-pair">
          <div className="material-specimen-alpha light">
            <img src={asset.uri} alt="" />
          </div>
          <div className="material-specimen-alpha dark">
            <img src={asset.uri} alt="" />
          </div>
        </div>
        {asset.id !== 'frame.portrait.v2' && (
          <div
            className="material-specimen-stretch"
            style={{ borderImageSource: `url(${asset.uri})` }}
          >
            <Field label="Мира Mira — Ёжик № 12">
              {(props) => <input {...props} placeholder="Действие героя / Hero action" />}
            </Field>
            <Button>Продолжить / Continue</Button>
          </div>
        )}
      </section>
    ))}
  </main>,
);
