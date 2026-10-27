// Geräteprofile (AKL-M040-02/06). Spiegelt die Antworten von /api/v1/devices.

export type DeviceKind = 'ai_key' | 'ai_port';
export type DeviceState =
	| 'unconfigured'
	| 'discovered'
	| 'pending_adoption'
	| 'adopted'
	| 'online'
	| 'reconnecting'
	| 'offline'
	| 'error'
	| 'disabled';

export interface DeviceProfile {
	id: string;
	kind: DeviceKind;
	state: DeviceState;
	display_name: string;
	/** SHA-256 des Gerätezertifikats; `null` ohne Identität. */
	fingerprint: string | null;
	created_at: string;
	updated_at: string;
}

export interface KillSwitch {
	active: boolean;
	/** Per AKL_DEVICE_KILL_SWITCH erzwungen – per UI nicht abschaltbar. */
	forced: boolean;
	changed_at: string | null;
	changed_by: string | null;
}

export interface DeviceList {
	kill_switch: KillSwitch;
	profiles: DeviceProfile[];
}

export interface Transition {
	id: number;
	from_state: string;
	to_state: string;
	reason: string | null;
	actor: string;
	correlation_id: string | null;
	occurred_at: string;
}

/**
 * Kompakte Signatur der Gerätezustände aus dem SSE-Status. Ändert sie sich,
 * lädt die Geräteseite die Profile neu – kein eigenes Polling.
 */
export function deviceSignature(
	devices: ReadonlyArray<{ profile: string; state: string | null }> | undefined
): string | null {
	if (!devices) return null;
	return devices
		.map((d) => `${d.profile}:${d.state ?? 'null'}`)
		.sort()
		.join('|');
}
