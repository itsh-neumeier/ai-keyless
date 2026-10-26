import { ApiError, api, setCsrfToken } from '#lib/api/client.ts';

export type Role = 'viewer' | 'operator' | 'admin';

export interface UserView {
	id: string;
	username: string;
	role: Role;
	disabled: boolean;
}

interface SessionView {
	user: UserView;
	csrf_token: string;
}

export type AuthPhase = 'loading' | 'setup' | 'login' | 'ready' | 'unavailable';

const RANK: Record<Role, number> = { viewer: 0, operator: 1, admin: 2 };

export function roleAllows(role: Role, required: Role): boolean {
	return RANK[role] >= RANK[required];
}

class Auth {
	phase = $state<AuthPhase>('loading');
	user = $state<UserView | null>(null);
	/** Fehlercode der letzten Prüfung (z. B. `database_unavailable`). */
	reason = $state<string | null>(null);

	#apply(session: SessionView) {
		setCsrfToken(session.csrf_token);
		this.user = session.user;
		this.reason = null;
		this.phase = 'ready';
	}

	#clear(phase: AuthPhase) {
		setCsrfToken(null);
		this.user = null;
		this.phase = phase;
	}

	/** Ermittelt den Anmeldezustand neu (Start, nach Verbindungsabbruch). */
	async refresh() {
		try {
			const state = await api<{ setup_required: boolean | null }>('/api/v1/auth/state');
			if (state.setup_required) return this.#clear('setup');
			this.#apply(await api<SessionView>('/api/v1/auth/session'));
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) return this.#clear('login');
			this.reason = err instanceof ApiError ? err.code : 'network';
			this.#clear('unavailable');
		}
	}

	async setup(token: string, username: string, password: string) {
		this.#apply(
			await api<SessionView>('/api/v1/auth/setup', {
				method: 'POST',
				body: { token, username, password }
			})
		);
	}

	async login(username: string, password: string) {
		this.#apply(
			await api<SessionView>('/api/v1/auth/login', {
				method: 'POST',
				body: { username, password }
			})
		);
	}

	async logout() {
		try {
			await api('/api/v1/auth/logout', { method: 'POST' });
		} finally {
			this.#clear('login');
		}
	}

	can(required: Role): boolean {
		return this.user !== null && roleAllows(this.user.role, required);
	}
}

export const auth = new Auth();
