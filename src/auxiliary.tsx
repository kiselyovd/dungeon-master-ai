import React from 'react';
import ReactDOM from 'react-dom/client';
import { AuxiliaryApp } from './app/AuxiliaryApp';
import './styles/globals.css';
import './styles/combat.css';
import './styles/living-tabletop.css';
import './i18n';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <AuxiliaryApp />
  </React.StrictMode>,
);
