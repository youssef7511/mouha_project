import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import process from 'node:process';
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    host: process.env.TAURI_DEV_HOST || '127.0.0.1',
    proxy: { '/api': { target: process.env.PACKFLOW_MOBILE_CORE_URL || 'http://127.0.0.1:4328' } },
  },
  build: { target: 'es2022' },
});
