import { describe, expect, it } from 'vitest';
import { parseSystemStatus } from './types';

const valid = {
	service: 'ai-keyless',
	version: '0.1.0',
	unofficial: true,
	database: { status: 'not_configured', schema_version: null },
	devices: [
		{ profile: 'ai_key', state: null, lifecycle_evidence: 'planned' },
		{ profile: 'ai_port', state: 'online', lifecycle_evidence: 'native_verified' }
	]
};

describe('parseSystemStatus', () => {
	it('akzeptiert den API-Vertrag', () => {
		const s = parseSystemStatus(valid);
		expect(s.devices[0]?.state).toBeNull();
		expect(s.database.status).toBe('not_configured');
	});

	it('lehnt unbekannte Zustände ab statt sie anzuzeigen', () => {
		const bad = structuredClone(valid);
		bad.devices[0]!.state = 'ready' as never;
		expect(() => parseSystemStatus(bad)).toThrow();
	});

	it('lehnt Antworten ohne Inoffiziell-Kennzeichnung ab', () => {
		expect(() => parseSystemStatus({ ...valid, unofficial: false })).toThrow();
	});

	it('lehnt Nicht-Objekte ab', () => {
		expect(() => parseSystemStatus(null)).toThrow();
		expect(() => parseSystemStatus('ok')).toThrow();
	});
});
