// Fetch-Wrapper: gleiche Herkunft, Cookies, CSRF-Header für schreibende Anfragen,
// einheitliche Fehler aus `{"error": {"code", "message"}}`.

export class ApiError extends Error {
	constructor(
		readonly status: number,
		readonly code: string,
		message: string
	) {
		super(message);
	}
}

let csrfToken: string | null = null;

export function setCsrfToken(token: string | null) {
	csrfToken = token;
}

export async function api<T>(
	path: string,
	options: { method?: string; body?: unknown } = {}
): Promise<T> {
	const method = options.method ?? 'GET';
	const headers: Record<string, string> = { accept: 'application/json' };
	if (options.body !== undefined) headers['content-type'] = 'application/json';
	if (method !== 'GET' && csrfToken) headers['x-csrf-token'] = csrfToken;

	let res: Response;
	try {
		res = await fetch(path, {
			method,
			headers,
			credentials: 'same-origin',
			body: options.body === undefined ? undefined : JSON.stringify(options.body)
		});
	} catch {
		throw new ApiError(0, 'network', 'API nicht erreichbar');
	}
	if (res.status === 204) return undefined as T;
	let data: unknown = null;
	try {
		data = await res.json();
	} catch {
		/* keine JSON-Antwort */
	}
	if (!res.ok) {
		const err = (data as { error?: { code?: unknown; message?: unknown } } | null)?.error;
		throw new ApiError(
			res.status,
			typeof err?.code === 'string' ? err.code : 'http_' + res.status,
			typeof err?.message === 'string' ? err.message : res.statusText
		);
	}
	return data as T;
}
