import { describe, expect, it } from 'vitest';
import { jobActions } from './jobs.ts';

describe('jobActions', () => {
	it('laufende und abgeschlossene Jobs sind unantastbar', () => {
		expect(jobActions('running')).toEqual({ cancel: false, requeue: false });
		expect(jobActions('completed')).toEqual({ cancel: false, requeue: false });
		expect(jobActions('cancelled')).toEqual({ cancel: false, requeue: false });
	});

	it('uncertain nur manuell neu einreihen', () => {
		expect(jobActions('uncertain')).toEqual({ cancel: true, requeue: true });
		expect(jobActions('retryable')).toEqual({ cancel: true, requeue: false });
	});
});
