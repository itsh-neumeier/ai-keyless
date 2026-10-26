import { describe, expect, it } from 'vitest';
import { roleAllows } from './auth.svelte.ts';

describe('Rollen', () => {
	it('folgen der Hierarchie viewer < operator < admin', () => {
		expect(roleAllows('admin', 'operator')).toBe(true);
		expect(roleAllows('operator', 'viewer')).toBe(true);
		expect(roleAllows('viewer', 'operator')).toBe(false);
		expect(roleAllows('operator', 'admin')).toBe(false);
	});
});
