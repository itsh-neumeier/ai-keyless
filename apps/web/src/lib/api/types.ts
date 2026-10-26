// Spiegel der Rust-Verträge in crates/core/src/status.rs.
// Änderungen immer gemeinsam mit docs/api/openapi.yaml vornehmen.

export type EvidenceLevel =
	| 'planned'
	| 'implemented'
	| 'synthetic_verified'
	| 'native_verified'
	| 'unsupported'
	| 'needs_evidence';

export type DeviceProfileKind = 'ai_key' | 'ai_port';

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

export type ComponentStatus = 'ok' | 'degraded' | 'unavailable' | 'not_configured';

export interface DatabaseStatus {
	status: ComponentStatus;
	schema_version: number | null;
}

export interface DeviceProfileStatus {
	profile: DeviceProfileKind;
	/** null = unbekannt bzw. kein Profil angelegt. Nie erfinden. */
	state: DeviceState | null;
	lifecycle_evidence: EvidenceLevel;
}

export interface SystemStatus {
	service: string;
	version: string;
	unofficial: boolean;
	database: DatabaseStatus;
	devices: DeviceProfileStatus[];
}

const EVIDENCE: readonly EvidenceLevel[] = [
	'planned',
	'implemented',
	'synthetic_verified',
	'native_verified',
	'unsupported',
	'needs_evidence'
];
const COMPONENT: readonly ComponentStatus[] = ['ok', 'degraded', 'unavailable', 'not_configured'];
const STATES: readonly DeviceState[] = [
	'unconfigured',
	'discovered',
	'pending_adoption',
	'adopted',
	'online',
	'reconnecting',
	'offline',
	'error',
	'disabled'
];

function isObject(v: unknown): v is Record<string, unknown> {
	return typeof v === 'object' && v !== null;
}

/**
 * Laufzeitprüfung der API-Antwort. Unerwartete Formen werden abgelehnt,
 * statt sie als gültigen Status anzuzeigen.
 */
export function parseSystemStatus(input: unknown): SystemStatus {
	if (!isObject(input)) throw new Error('Status: kein Objekt');
	const { service, version, unofficial, database, devices } = input;
	if (typeof service !== 'string' || typeof version !== 'string')
		throw new Error('Status: service/version fehlen');
	if (unofficial !== true) throw new Error('Status: unofficial muss true sein');
	if (!isObject(database) || !COMPONENT.includes(database.status as ComponentStatus))
		throw new Error('Status: database ungültig');
	const schema = database.schema_version;
	if (schema !== null && typeof schema !== 'number')
		throw new Error('Status: schema_version ungültig');
	if (!Array.isArray(devices)) throw new Error('Status: devices fehlt');
	const parsedDevices = devices.map((d): DeviceProfileStatus => {
		if (!isObject(d)) throw new Error('Status: Gerät ungültig');
		if (d.profile !== 'ai_key' && d.profile !== 'ai_port')
			throw new Error('Status: Profil ungültig');
		if (d.state !== null && !STATES.includes(d.state as DeviceState))
			throw new Error('Status: Gerätezustand ungültig');
		if (!EVIDENCE.includes(d.lifecycle_evidence as EvidenceLevel))
			throw new Error('Status: Evidenz ungültig');
		return {
			profile: d.profile,
			state: d.state as DeviceState | null,
			lifecycle_evidence: d.lifecycle_evidence as EvidenceLevel
		};
	});
	return {
		service,
		version,
		unofficial,
		database: { status: database.status as ComponentStatus, schema_version: schema },
		devices: parsedDevices
	};
}
