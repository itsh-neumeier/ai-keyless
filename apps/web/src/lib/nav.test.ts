import { describe, expect, it } from 'vitest';
import { NAV, findNav } from './nav';

describe('Navigation', () => {
	it('enthält die zehn Hauptbereiche in fester Reihenfolge', () => {
		expect(NAV.map((i) => i.id)).toEqual([
			'overview',
			'devices',
			'cameras',
			'detections',
			'search',
			'models',
			'processing',
			'alerts',
			'system',
			'settings'
		]);
	});

	it('ordnet Unterpfade dem Bereich zu', () => {
		expect(findNav('/')?.id).toBe('overview');
		expect(findNav('/cameras/abc')?.id).toBe('cameras');
		expect(findNav('/unbekannt')).toBeUndefined();
	});

	it('jeder Bereich hat einen Meilenstein', () => {
		for (const item of NAV) expect(item.milestone).toMatch(/^AKL-M\d{3}$/);
	});
});

describe('Routing-Vertrag', () => {
	it('implementierte Bereiche haben eigene Routen, alle anderen nutzen den Platzhalter', () => {
		const ownRoutes = ['/', '/devices', '/models', '/processing', '/system', '/settings'];
		for (const item of NAV) expect(ownRoutes.includes(item.href)).toBe(item.implemented);
	});
});
