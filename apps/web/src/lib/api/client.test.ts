import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, api, setCsrfToken } from './client.ts';

afterEach(() => {
	vi.unstubAllGlobals();
	setCsrfToken(null);
});

function stubFetch(status: number, body: unknown) {
	const fn = vi.fn(
		async () =>
			new Response(body === undefined ? null : JSON.stringify(body), {
				status,
				headers: { 'content-type': 'application/json' }
			})
	);
	vi.stubGlobal('fetch', fn);
	return fn;
}

describe('api()', () => {
	it('sendet CSRF-Token nur bei schreibenden Anfragen', async () => {
		const fn = stubFetch(200, { ok: true });
		setCsrfToken('tok');
		await api('/x');
		await api('/y', { method: 'POST', body: { a: 1 } });
		const headers = (call: number) =>
			(fn.mock.calls[call] as unknown as [string, RequestInit])[1].headers as Record<
				string,
				string
			>;
		expect(headers(0)['x-csrf-token']).toBeUndefined();
		expect(headers(1)['x-csrf-token']).toBe('tok');
	});

	it('wandelt Fehlerantworten in ApiError mit Code', async () => {
		stubFetch(401, { error: { code: 'invalid_credentials', message: 'falsch' } });
		await expect(api('/login', { method: 'POST', body: {} })).rejects.toMatchObject({
			status: 401,
			code: 'invalid_credentials'
		});
	});

	it('meldet Netzwerkfehler als code=network', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn(async () => {
				throw new TypeError('failed');
			})
		);
		const err: unknown = await api('/x').catch((e: unknown) => e);
		expect(err).toBeInstanceOf(ApiError);
		expect((err as ApiError).code).toBe('network');
	});
});
