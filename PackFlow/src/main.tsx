import React from 'react';
import ReactDOM from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { App } from './app/App';
import './styles/app.css';
import { LocalAccount } from './features/LocalAccount';
const client = new QueryClient({
  defaultOptions: { queries: { retry: false, staleTime: 15_000 }, mutations: { retry: false } },
});
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={client}>
      <LocalAccount>
        <App />
      </LocalAccount>
    </QueryClientProvider>
  </React.StrictMode>,
);
