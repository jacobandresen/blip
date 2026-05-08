import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: 'test/e2e',
  timeout: 30_000,
  use: {
    baseURL: 'http://localhost:8080',
  },
  webServer: {
    command: 'python3 -m http.server -d web 8080',
    url: 'http://localhost:8080',
    reuseExistingServer: true,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
