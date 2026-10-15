import { defineConfig } from '@playwright/test';
import process from 'node:process';
export default defineConfig({
  testDir: './tests/browser',
  workers: 1,
  fullyParallel: false,
  use: {
    baseURL: 'http://127.0.0.1:1421',
    viewport: { width: 1440, height: 900 },
    trace: 'retain-on-failure',
    launchOptions: process.env.PACKFLOW_CHROMIUM_PATH
      ? { executablePath: process.env.PACKFLOW_CHROMIUM_PATH }
      : {},
  },
  webServer: {
    command: 'npm run dev -- --port 1421',
    url: 'http://127.0.0.1:1421',
    reuseExistingServer: false,
    env: { PACKFLOW_CORE_URL: process.env.PACKFLOW_CORE_URL ?? 'http://127.0.0.1:4319' },
  },
});
