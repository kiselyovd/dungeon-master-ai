import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { startProviderStartup } from './providerStartup';

export function bootstrap(root: HTMLElement): void {
  const disposeProviderStartup = startProviderStartup();
  window.addEventListener('pagehide', disposeProviderStartup, { once: true });
  import.meta.hot?.dispose(disposeProviderStartup);
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
