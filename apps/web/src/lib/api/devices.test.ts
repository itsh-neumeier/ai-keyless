import { describe, expect, it } from 'vitest';
import { canForgetController, deviceSignature } from './devices.ts';

describe('deviceSignature()', () => {
	it('ist ohne Daten null (kein vorgetäuschter Zustand)', () => {
		expect(deviceSignature(undefined)).toBeNull();
	});

	it('ändert sich mit dem Zustand, nicht mit der Reihenfolge', () => {
		const a = deviceSignature([
			{ profile: 'ai_key', state: 'unconfigured' },
			{ profile: 'ai_port', state: null }
		]);
		const b = deviceSignature([
			{ profile: 'ai_port', state: null },
			{ profile: 'ai_key', state: 'unconfigured' }
		]);
		const c = deviceSignature([
			{ profile: 'ai_key', state: 'disabled' },
			{ profile: 'ai_port', state: null }
		]);
		expect(a).toBe(b);
		expect(a).not.toBe(c);
	});
});

describe('canForgetController()', () => {
	const controller = { ip: '192.0.2.10', port: 7442, fingerprint: 'AB' };

	it('nur bei deaktiviertem Profil mit Bindung', () => {
		expect(canForgetController({ state: 'disabled', controller })).toBe(true);
		expect(canForgetController({ state: 'online', controller })).toBe(false);
		expect(canForgetController({ state: 'disabled', controller: null })).toBe(false);
	});
});
