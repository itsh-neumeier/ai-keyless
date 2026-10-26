import type { ComponentStatus, DeviceState, EvidenceLevel } from '#lib/api/types.ts';
import type { LinkState } from '#lib/api/status.svelte.ts';

export type Tone = 'ok' | 'warn' | 'danger' | 'neutral' | 'accent';

export function componentTone(status: ComponentStatus | null): Tone {
	switch (status) {
		case 'ok':
			return 'ok';
		case 'degraded':
			return 'warn';
		case 'unavailable':
			return 'danger';
		default:
			return 'neutral';
	}
}

export function deviceTone(state: DeviceState | null): Tone {
	switch (state) {
		case 'online':
			return 'ok';
		case 'adopted':
		case 'discovered':
		case 'pending_adoption':
			return 'accent';
		case 'reconnecting':
		case 'offline':
			return 'warn';
		case 'error':
			return 'danger';
		default:
			return 'neutral';
	}
}

export function evidenceTone(level: EvidenceLevel): Tone {
	switch (level) {
		case 'native_verified':
			return 'ok';
		case 'synthetic_verified':
			return 'accent';
		case 'needs_evidence':
			return 'warn';
		case 'unsupported':
			return 'danger';
		default:
			return 'neutral';
	}
}

export function linkTone(link: LinkState): Tone {
	return link === 'live' ? 'ok' : link === 'connecting' ? 'neutral' : 'danger';
}
