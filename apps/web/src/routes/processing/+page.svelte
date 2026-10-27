<script lang="ts">
	import { ApiError, api } from '#lib/api/client.ts';
	import { JOB_STATES, jobActions, type Job, type JobState, type JobStats } from '#lib/api/jobs.ts';
	import { auth } from '#lib/auth/auth.svelte.ts';
	import Button from '#lib/components/Button.svelte';
	import ConfirmButton from '#lib/components/ConfirmButton.svelte';
	import FormError from '#lib/components/FormError.svelte';
	import StatusChip from '#lib/components/StatusChip.svelte';
	import type { Tone } from '#lib/components/tone.ts';
	import { errorText, i18n, t, type MessageKey } from '#lib/i18n/index.svelte.ts';

	// Sanfte Aktualisierung nur bei sichtbarer Seite (kein Polling-Sturm);
	// ein Ereignis-Stream für Jobs folgt mit AKL-M070.
	const REFRESH_MS = 10_000;

	let stats = $state<JobStats | null>(null);
	let jobs = $state<Job[] | null>(null);
	let filter = $state<JobState | ''>('');
	let error = $state<string | null>(null);
	let loading = $state(false);
	let busy = $state<Record<string, boolean>>({});

	const canAct = $derived(auth.can('operator'));
	const STATE_TONE: Record<JobState, Tone> = {
		queued: 'neutral',
		running: 'accent',
		completed: 'ok',
		failed: 'danger',
		retryable: 'warn',
		uncertain: 'warn',
		cancelled: 'neutral'
	};

	async function load() {
		loading = true;
		try {
			const query = filter ? `?state=${filter}&limit=100` : '?limit=100';
			[stats, jobs] = await Promise.all([
				api<JobStats>('/api/v1/jobs/stats'),
				api<Job[]>(`/api/v1/jobs${query}`)
			]);
			error = null;
		} catch (err) {
			error = errorText(err instanceof ApiError ? err.code : 'unknown');
		} finally {
			loading = false;
		}
	}

	async function act(job: Job, action: 'cancel' | 'requeue') {
		busy[job.id] = true;
		try {
			await api(`/api/v1/jobs/${job.id}/${action}`, { method: 'POST' });
			await load();
		} catch (err) {
			error = errorText(err instanceof ApiError ? err.code : 'unknown');
		} finally {
			busy[job.id] = false;
		}
	}

	function when(iso: string | null) {
		return iso ? new Date(iso).toLocaleString(i18n.locale === 'de' ? 'de-DE' : 'en-GB') : '—';
	}

	function detail(job: Job): string {
		if (job.last_error_code) return errorText(job.last_error_code);
		const r = job.result;
		if (!r) return '—';
		const parts = [
			r.model,
			r.latency_ms !== undefined ? `${r.latency_ms} ms` : null,
			r.text ? `„${r.text}“` : null
		];
		return parts.filter(Boolean).join(' · ');
	}

	$effect(() => {
		void filter;
		load();
	});

	$effect(() => {
		const timer = setInterval(() => {
			if (document.visibilityState === 'visible') load();
		}, REFRESH_MS);
		return () => clearInterval(timer);
	});

	const usage = $derived(
		stats && stats.limit_per_hour > 0
			? Math.min(100, Math.round((stats.starts_last_hour / stats.limit_per_hour) * 100))
			: 100
	);
</script>

<p class="mb-5 text-sm text-muted">{t('processing.subtitle')}</p>
<FormError message={error} />

<div class="grid gap-4 lg:grid-cols-3">
	<section class="rounded-xl border border-line bg-surface p-4 shadow-card" aria-labelledby="limit">
		<h2 id="limit" class="text-xs font-medium tracking-wide text-muted uppercase">
			{t('processing.limit')}
		</h2>
		{#if stats}
			<p class="mt-2 font-mono text-2xl tabular-nums">
				{stats.starts_last_hour}<span class="text-base text-faint"> / {stats.limit_per_hour}</span>
			</p>
			<div
				class="mt-3 h-1.5 overflow-hidden rounded-full bg-surface-2"
				role="progressbar"
				aria-valuemin="0"
				aria-valuemax={stats.limit_per_hour}
				aria-valuenow={stats.starts_last_hour}
				aria-label={t('processing.limit')}
			>
				<div
					class="h-full rounded-full {usage >= 100 ? 'bg-warn' : 'bg-accent'}"
					style:width="{usage}%"
				></div>
			</div>
			<p class="mt-3 text-xs text-faint">{t('processing.limitHint')}</p>
		{:else}
			<div class="mt-3 h-8 w-24 animate-pulse rounded bg-surface-2" aria-hidden="true"></div>
		{/if}
	</section>

	<section
		class="rounded-xl border border-line bg-surface p-4 shadow-card lg:col-span-2"
		aria-labelledby="counts"
	>
		<h2 id="counts" class="sr-only">{t('processing.filter')}</h2>
		<div class="flex flex-wrap gap-2">
			{#each JOB_STATES as s (s)}
				<button
					type="button"
					class="rounded-lg border px-3 py-2 text-left transition {filter === s
						? 'border-accent bg-accent-soft'
						: 'border-line hover:bg-surface-hover'}"
					aria-pressed={filter === s}
					onclick={() => (filter = filter === s ? '' : s)}
				>
					<span class="block text-[11px] text-muted">{t(`job.state.${s}` as MessageKey)}</span>
					<span class="block font-mono text-lg tabular-nums">{stats?.counts[s] ?? '—'}</span>
				</button>
			{/each}
		</div>
		<p class="mt-3 text-xs text-muted">
			{t('processing.oldest')}:
			<span class="font-mono">{when(stats?.oldest_ready_at ?? null)}</span>
		</p>
	</section>
</div>

<p class="mt-4 text-xs text-faint">{t('processing.noProducer')}</p>

<section class="mt-4 rounded-xl border border-line bg-surface shadow-card" aria-labelledby="jobs">
	<div class="flex flex-wrap items-center gap-3 border-b border-line px-4 py-3">
		<h2 id="jobs" class="text-sm font-semibold">{t('nav.processing')}</h2>
		<label class="ml-auto flex items-center gap-2 text-xs text-muted">
			{t('processing.filter')}
			<select
				bind:value={filter}
				class="h-8 rounded-md border border-line bg-surface-2 px-2 text-[13px] text-fg"
			>
				<option value="">{t('processing.all')}</option>
				{#each JOB_STATES as s (s)}
					<option value={s}>{t(`job.state.${s}` as MessageKey)}</option>
				{/each}
			</select>
		</label>
		<Button variant="secondary" busy={loading} onclick={load}>{t('processing.refresh')}</Button>
	</div>
	<div class="relative overflow-x-auto">
		<table class="w-full min-w-[760px] text-[13px]">
			<thead class="text-left text-xs text-faint">
				<tr>
					<th scope="col" class="px-4 py-2 font-medium">{t('processing.created')}</th>
					<th scope="col" class="px-4 py-2 font-medium">{t('processing.kind')}</th>
					<th scope="col" class="px-4 py-2 font-medium">{t('processing.filter')}</th>
					<th scope="col" class="px-4 py-2 font-medium">{t('processing.attempts')}</th>
					<th scope="col" class="px-4 py-2 font-medium">{t('processing.detail')}</th>
					{#if canAct}<th scope="col" class="px-4 py-2"><span class="sr-only">Aktionen</span></th
						>{/if}
				</tr>
			</thead>
			<tbody>
				{#if jobs?.length === 0}
					<tr class="border-t border-line">
						<td colspan="6" class="px-4 py-6 text-center text-muted">{t('processing.none')}</td>
					</tr>
				{/if}
				{#each jobs ?? [] as job (job.id)}
					{@const actions = jobActions(job.state)}
					<tr class="border-t border-line align-top hover:bg-surface-hover">
						<td class="px-4 py-2.5 font-mono text-xs whitespace-nowrap">{when(job.created_at)}</td>
						<td class="px-4 py-2.5 font-mono text-xs">{job.kind}</td>
						<td class="px-4 py-2.5">
							<StatusChip
								tone={STATE_TONE[job.state]}
								label={t(`job.state.${job.state}` as MessageKey)}
							/>
						</td>
						<td class="px-4 py-2.5 font-mono text-xs tabular-nums"
							>{job.attempts}/{job.max_attempts}</td
						>
						<td class="px-4 py-2.5 text-xs text-muted">{detail(job)}</td>
						{#if canAct}
							<td class="px-4 py-2 text-right whitespace-nowrap">
								{#if actions.requeue}
									<ConfirmButton
										variant="ghost"
										confirmLabel={t('processing.requeueConfirm')}
										busy={busy[job.id]}
										onconfirm={() => act(job, 'requeue')}>{t('processing.requeue')}</ConfirmButton
									>
								{/if}
								{#if actions.cancel}
									<ConfirmButton
										variant="ghost"
										confirmLabel={t('processing.cancelConfirm')}
										busy={busy[job.id]}
										onconfirm={() => act(job, 'cancel')}>{t('processing.cancel')}</ConfirmButton
									>
								{/if}
							</td>
						{/if}
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
	<p class="border-t border-line px-4 py-2.5 text-xs text-faint">{t('processing.uncertainHint')}</p>
</section>
