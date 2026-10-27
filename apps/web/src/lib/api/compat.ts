import type { DeviceProfileKind, EvidenceLevel } from './types.ts';

export type Capability =
	| 'discovery'
	| 'adoption'
	| 'control_channel'
	| 'keepalive'
	| 'caption_on_demand'
	| 'caption_automatic'
	| 'native_search'
	| 'ai_alerts'
	| 'speech_to_text'
	| 'license_plates'
	| 'faces'
	| 'enhancement'
	| 'port_pairing'
	| 'port_stream_processing'
	| 'port_smart_detections'
	| 'port_smart_zones';

export interface Claim {
	capability: Capability;
	evidence: EvidenceLevel;
	note: string;
}

export interface VersionRange {
	id: string;
	min: string;
	max_exclusive: string;
	source: string;
	claims: Claim[];
}

export interface CompatProfiles {
	profiles: { profile: DeviceProfileKind; ranges: VersionRange[] }[];
}
