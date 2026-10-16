import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';

export function bootstrap(root: HTMLElement): void {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
