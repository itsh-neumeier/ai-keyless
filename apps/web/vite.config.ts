import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';

const apiTarget = process.env.AKL_API_URL ?? 'http://127.0.0.1:8090';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			// Statische SPA, ausgeliefert von der Rust-API (ADR-001).
			adapter: adapter({ fallback: 'index.html', strict: true })
		})
	],
	server: {
		proxy: {
			'/api': apiTarget,
			'/healthz': apiTarget,
			'/readyz': apiTarget
		}
	},
	test: {
		include: ['src/**/*.test.ts'],
		environment: 'node'
	}
});
