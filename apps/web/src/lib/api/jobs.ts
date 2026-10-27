export type JobState =
	'queued' | 'running' | 'completed' | 'failed' | 'retryable' | 'uncertain' | 'cancelled';

export const JOB_STATES: readonly JobState[] = [
	'queued',
	'running',
	'retryable',
	'completed',
	'failed',
	'uncertain',
	'cancelled'
];

export interface Job {
	id: string;
	idempotency_key: string;
	kind: string;
	state: JobState;
	phase: string;
	priority: number;
	attempts: number;
	max_attempts: number;
	next_attempt_at: string;
	last_error_code: string | null;
	payload: Record<string, unknown>;
	result: Record<string, unknown> | null;
	created_at: string;
	started_at: string | null;
	finished_at: string | null;
}

export interface JobStats {
	counts: Record<JobState, number>;
	oldest_ready_at: string | null;
	starts_last_hour: number;
	limit_per_hour: number;
}

/** Welche Operator-Aktionen sind im Zustand erlaubt (Spiegel von core::job). */
export function jobActions(state: JobState): { cancel: boolean; requeue: boolean } {
	return {
		cancel: ['queued', 'retryable', 'uncertain', 'failed'].includes(state),
		requeue: state === 'failed' || state === 'uncertain'
	};
}
