import { defineConfig, devices } from '@playwright/test';

// Läuft gegen einen bereits gestarteten Stack (scripts/e2e.sh).
export default defineConfig({
	testDir: './specs',
	fullyParallel: false,
	workers: 1,
	retries: 0,
	timeout: 60_000,
	reporter: [['list']],
	use: {
		baseURL: process.env.E2E_BASE_URL ?? 'http://127.0.0.1:18090',
		trace: 'retain-on-failure',
		colorScheme: 'dark',
		locale: 'de-DE'
	},
	projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }]
});
