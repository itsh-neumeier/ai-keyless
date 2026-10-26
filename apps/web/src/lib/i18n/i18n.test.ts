import { describe, expect, it } from 'vitest';
import { de } from './de';
import { en } from './en';

describe('i18n', () => {
	it('englische Übersetzung deckt alle deutschen Schlüssel ab', () => {
		expect(Object.keys(en).sort()).toEqual(Object.keys(de).sort());
	});

	it('keine leeren Texte', () => {
		for (const dict of [de, en]) {
			for (const [key, value] of Object.entries(dict)) expect(value, key).not.toBe('');
		}
	});

	it('jeder Gerätezustand und jede Evidenzstufe ist übersetzt', () => {
		const states = [
			'null',
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
		for (const s of states) expect(de).toHaveProperty(`device.state.${s}`);
		const levels = [
			'planned',
			'implemented',
			'synthetic_verified',
			'native_verified',
			'unsupported',
			'needs_evidence'
		];
		for (const l of levels) expect(de).toHaveProperty(`evidence.${l}`);
	});
});
