import { defineConfig } from '@playwright/test';
import process from 'node:process';
export default defineConfig({
  testDir: './tests',
  workers: 1,
  fullyParallel: false,
  use: {
    baseURL: 'http://127.0.0.1:1431',
    viewport: { width: 390, height: 844 },
    isMobile: true,
    hasTouch: true,
    trace: 'retain-on-failure',
    launchOptions: process.env.PACKFLOW_CHROMIUM_PATH
      ? { executablePath: process.env.PACKFLOW_CHROMIUM_PATH }
      : {},
  },
  webServer: {
    command: 'npm run dev -- --port 1431',
    url: 'http://127.0.0.1:1431',
    reuseExistingServer: false,
    env: { PACKFLOW_MOBILE_CORE_URL: 'http://127.0.0.1:4329' },
  },
});
