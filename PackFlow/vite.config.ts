import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwind from '@tailwindcss/vite';
import process from 'node:process';

export default defineConfig({
  plugins: [react(), tailwind()],
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: [
        '**/crates/**',
        '**/src-tauri/**',
        '**/.dev-data/**',
        '**/target/**',
        '**/planning/**',
        '**/test-results/**',
        '**/playwright-report/**',
      ],
    },
    proxy: { '/api': process.env.PACKFLOW_CORE_URL ?? 'http://127.0.0.1:4318' },
  },
  preview: {
    port: 1420,
    strictPort: true,
    proxy: { '/api': process.env.PACKFLOW_CORE_URL ?? 'http://127.0.0.1:4318' },
  },
  clearScreen: false,
});
