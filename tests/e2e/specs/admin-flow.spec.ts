import { expect, test, type Page } from '@playwright/test';
import { startMockOllama } from './mock-ollama.ts';

// Ein durchgehender Ablauf auf einer frischen Installation (Reihenfolge wichtig).
test.describe.configure({ mode: 'serial' });

const ADMIN = { user: 'e2e-admin', pass: 'e2e-admin-passwort' };
const VIEWER = { user: 'e2e-viewer', pass: 'e2e-viewer-passwort' };
const token = process.env.E2E_SETUP_TOKEN ?? '';
const providerHost = process.env.E2E_PROVIDER_HOST ?? 'host.docker.internal';

async function login(page: Page, user: string, pass: string) {
	await page.goto('/');
	await page.fill('#username', user);
	await page.fill('#password', pass);
	await page.click('button[type=submit]');
	await expect(page.getByRole('button', { name: 'Abmelden' })).toBeVisible();
}

async function noHorizontalOverflow(page: Page) {
	const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
	expect(overflow).toBeLessThanOrEqual(0);
}

test('Einrichtung nur mit gültigem Token, danach Übersicht ohne erfundene Zustände', async ({
	page
}) => {
	expect(token, 'E2E_SETUP_TOKEN fehlt').toHaveLength(43);
	await page.goto('/');
	await expect(page.getByRole('heading', { name: 'AI-Keyless einrichten' })).toBeVisible();

	await page.fill('#token', 'falsches-token');
	await page.fill('#username', ADMIN.user);
	await page.fill('#password', ADMIN.pass);
	await page.fill('#password-repeat', ADMIN.pass);
	await page.click('button[type=submit]');
	await expect(page.getByRole('alert')).toContainText('Einrichtungs-Token ungültig');

	await page.fill('#token', token);
	await page.click('button[type=submit]');
	await expect(page.getByRole('heading', { name: 'Geräteprofile' })).toBeVisible();
	await expect(page.locator('header')).toContainText('Live');
	const devices = page.locator('main table');
	await expect(devices).toContainText('Nicht eingerichtet');
	await expect(devices).not.toContainText('Online');
	await expect(devices).not.toContainText('Adoptiert');

	const cookie = (await page.context().cookies()).find((c) => c.name === 'akl_session');
	expect(cookie?.httpOnly).toBe(true);
	expect(cookie?.sameSite).toBe('Strict');
});

test('Rollen: Betrachter sieht keine Verwaltungsfunktionen', async ({ page }) => {
	await login(page, ADMIN.user, ADMIN.pass);
	await page.goto('/settings');
	await page.fill('#new-username', VIEWER.user);
	await page.fill('#new-password', VIEWER.pass);
	await page.click('form button[type=submit]');
	await expect(page.getByText('Benutzer angelegt.')).toBeVisible();
	await page.getByRole('button', { name: 'Abmelden' }).click();

	await page.fill('#username', VIEWER.user);
	await page.fill('#password', 'falsch-falsch-falsch');
	await page.click('button[type=submit]');
	await expect(page.getByRole('alert')).toContainText('Benutzername oder Passwort falsch');

	await login(page, VIEWER.user, VIEWER.pass);
	await page.goto('/settings');
	await expect(page.getByRole('heading', { name: 'Darstellung' })).toBeVisible();
	await expect(page.getByRole('heading', { name: 'Benutzer' })).toHaveCount(0);
	await page.goto('/models');
	await expect(page.getByRole('heading', { name: 'Provider anlegen' })).toHaveCount(0);
});

test('Provider: Sperre, Verbindung, Bildtest und Verarbeitung über die Queue', async ({
	page
}) => {
	const mock = await startMockOllama();
	try {
		await login(page, ADMIN.user, ADMIN.pass);
		await page.goto('/models');

		await page.fill('#p-name', 'Metadaten');
		await page.fill('#p-url', 'http://169.254.169.254');
		await page.click('form button[type=submit]');
		await expect(page.getByRole('alert')).toContainText('gesperrt');

		await page.fill('#p-name', 'Mock-Ollama');
		await page.fill('#p-url', `http://${providerHost}:${mock.port}`);
		await page.fill('#p-model', 'mock-vision:1b');
		await page.click('form button[type=submit]');

		await page.getByRole('button', { name: 'Verbindung testen' }).click();
		await expect(page.getByText('Erreichbar', { exact: true })).toBeVisible();
		await expect(page.getByText('mock-vision:1b').first()).toBeVisible();

		await page.getByRole('button', { name: 'Bildtest' }).click();
		await page.getByRole('button', { name: 'Inferenz starten' }).click();
		await expect(page.getByText('Korrekt erkannt', { exact: true })).toBeVisible();

		await page.getByRole('button', { name: 'Als Job einreihen' }).click();
		await expect(page.getByText('Bildtest als Job eingereiht')).toBeVisible();
		await page.goto('/processing');
		await expect(async () => {
			await page.getByRole('button', { name: 'Aktualisieren' }).click();
			await expect(page.locator('tbody')).toContainText('Abgeschlossen', { timeout: 1000 });
		}).toPass({ timeout: 30_000 });
		await expect(page.locator('tbody')).toContainText('vision_probe');
	} finally {
		mock.server.close();
	}
});

test('Responsiv: keine horizontale Seitenscrollbar auf dem Smartphone', async ({ page }) => {
	await page.setViewportSize({ width: 390, height: 844 });
	await login(page, ADMIN.user, ADMIN.pass);
	for (const path of ['/', '/devices', '/models', '/processing', '/settings', '/system']) {
		await page.goto(path);
		// Kein 'networkidle': der Status-Stream (SSE) bleibt dauerhaft offen.
		await expect(page.locator('main')).toBeVisible();
		await page.waitForTimeout(500);
		await noHorizontalOverflow(page);
	}
});
