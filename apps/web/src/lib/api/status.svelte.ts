import { parseSystemStatus, type SystemStatus } from './types';

export type LinkState = 'connecting' | 'live' | 'offline' | 'invalid';

/**
 * Echtzeitstatus über Server-Sent Events. Der Server sendet nur Änderungen;
 * die Wiederverbindung übernimmt EventSource selbst (kein Polling-Sturm).
 */
class StatusStream {
	data = $state<SystemStatus | null>(null);
	link = $state<LinkState>('connecting');
	lastUpdate = $state<Date | null>(null);
	#source: EventSource | null = null;
	#users = 0;

	connect(): () => void {
		this.#users += 1;
		if (!this.#source) this.#open();
		return () => {
			this.#users -= 1;
			if (this.#users === 0) this.#close();
		};
	}

	#open() {
		this.link = 'connecting';
		const source = new EventSource('/api/v1/system/status/stream');
		source.addEventListener('status', (event) => {
			try {
				this.data = parseSystemStatus(JSON.parse((event as MessageEvent<string>).data));
				this.link = 'live';
				this.lastUpdate = new Date();
			} catch (err) {
				console.warn('Ungültige Statusnachricht', err);
				this.link = 'invalid';
			}
		});
		source.onerror = () => {
			// API nicht erreichbar: letzter bekannter Stand wird als veraltet markiert.
			this.link = 'offline';
		};
		this.#source = source;
	}

	#close() {
		this.#source?.close();
		this.#source = null;
	}
}

export const systemStatus = new StatusStream();
