<script lang="ts">
	import { systemStatus } from '#lib/api/status.svelte.ts';
	import Icon from '#lib/components/Icon.svelte';
	import StatusChip from '#lib/components/StatusChip.svelte';
	import { componentTone, deviceTone, evidenceTone, linkTone } from '#lib/components/tone.ts';
	import { i18n, t, type MessageKey } from '#lib/i18n/index.svelte.ts';

	const status = $derived(systemStatus.data);
	const live = $derived(systemStatus.link === 'live');
	const dbStatus = $derived(live && status ? status.database.status : null);
	const updated = $derived(
		systemStatus.lastUpdate?.toLocaleTimeString(i18n.locale === 'de' ? 'de-DE' : 'en-GB') ?? '—'
	);
</script>

<div class="mb-5">
	<p class="text-sm text-muted">{t('overview.subtitle')}</p>
</div>

<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
	<section
		class="rounded-xl border border-line bg-surface p-4 shadow-card"
		aria-labelledby="card-api"
	>
		<div class="mb-3 flex items-center gap-2 text-muted">
			<Icon name="api" size={16} />
			<h2 id="card-api" class="text-xs font-medium tracking-wide uppercase">{t('overview.api')}</h2>
		</div>
		<StatusChip
			tone={linkTone(systemStatus.link)}
			label={t(`link.${systemStatus.link}` as MessageKey)}
		/>
		<dl class="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-[13px]">
			<dt class="text-muted">{t('overview.version')}</dt>
			<dd class="text-right font-mono tabular-nums">{status?.version ?? '—'}</dd>
			<dt class="text-muted">{t('overview.lastUpdate')}</dt>
			<dd class="text-right font-mono tabular-nums">{updated}</dd>
		</dl>
	</section>

	<section
		class="rounded-xl border border-line bg-surface p-4 shadow-card"
		aria-labelledby="card-db"
	>
		<div class="mb-3 flex items-center gap-2 text-muted">
			<Icon name="database" size={16} />
			<h2 id="card-db" class="text-xs font-medium tracking-wide uppercase">
				{t('overview.database')}
			</h2>
		</div>
		{#if status === null}
			<div class="h-5 w-28 animate-pulse rounded-full bg-surface-2" aria-hidden="true"></div>
		{:else}
			<StatusChip
				tone={componentTone(dbStatus)}
				label={dbStatus ? t(`status.${dbStatus}` as MessageKey) : t('status.unknown')}
			/>
		{/if}
		<dl class="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-[13px]">
			<dt class="text-muted">{t('overview.schema')}</dt>
			<dd class="text-right font-mono tabular-nums">
				{live ? (status?.database.schema_version ?? '—') : '—'}
			</dd>
		</dl>
	</section>

	<section
		class="rounded-xl border border-line bg-surface p-4 shadow-card sm:col-span-2 xl:col-span-1"
		aria-labelledby="card-about"
	>
		<div class="mb-3 flex items-center gap-2 text-muted">
			<Icon name="info" size={16} />
			<h2 id="card-about" class="text-xs font-medium tracking-wide uppercase">
				{t('app.unofficial')}
			</h2>
		</div>
		<p class="text-[13px] text-muted">{t('app.unofficialLong')}</p>
	</section>
</div>

<section
	class="mt-6 rounded-xl border border-line bg-surface shadow-card"
	aria-labelledby="devices"
>
	<div class="flex items-center justify-between border-b border-line px-4 py-3">
		<h2 id="devices" class="text-sm font-semibold">{t('overview.devices')}</h2>
	</div>
	<div class="overflow-x-auto">
		<table class="w-full min-w-[480px] text-[13px]">
			<thead class="text-left text-xs text-faint">
				<tr>
					<th scope="col" class="px-4 py-2 font-medium">{t('overview.profile')}</th>
					<th scope="col" class="px-4 py-2 font-medium">{t('overview.state')}</th>
					<th scope="col" class="px-4 py-2 font-medium" title={t('evidence.tooltip')}>
						{t('overview.lifecycle')}
					</th>
				</tr>
			</thead>
			<tbody>
				{#if status === null}
					{#each [0, 1] as i (i)}
						<tr class="border-t border-line">
							<td class="px-4 py-3" colspan="3">
								<div class="h-4 w-2/3 animate-pulse rounded bg-surface-2" aria-hidden="true"></div>
							</td>
						</tr>
					{/each}
				{:else}
					{#each status.devices as device (device.profile)}
						{@const state = live ? device.state : null}
						<tr class="border-t border-line hover:bg-surface-hover">
							<td class="px-4 py-2.5 font-medium">{t(`device.${device.profile}` as MessageKey)}</td>
							<td class="px-4 py-2.5">
								<StatusChip
									tone={deviceTone(state)}
									label={live
										? t(`device.state.${state ?? 'null'}` as MessageKey)
										: t('status.unknown')}
								/>
							</td>
							<td class="px-4 py-2.5">
								<StatusChip
									tone={evidenceTone(device.lifecycle_evidence)}
									label={t(`evidence.${device.lifecycle_evidence}` as MessageKey)}
									title={t('evidence.tooltip')}
								/>
							</td>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</div>
	<p class="border-t border-line px-4 py-2.5 text-xs text-faint">{t('overview.noFake')}</p>
</section>
