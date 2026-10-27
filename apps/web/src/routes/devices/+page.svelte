<script lang="ts">
	import { ApiError, api } from '#lib/api/client.ts';
	import type { CompatProfiles } from '#lib/api/compat.ts';
	import { systemStatus } from '#lib/api/status.svelte.ts';
	import FormError from '#lib/components/FormError.svelte';
	import StatusChip from '#lib/components/StatusChip.svelte';
	import { deviceTone, evidenceTone } from '#lib/components/tone.ts';
	import { errorText, t, type MessageKey } from '#lib/i18n/index.svelte.ts';

	let compat = $state<CompatProfiles | null>(null);
	let error = $state<string | null>(null);

	$effect(() => {
		api<CompatProfiles>('/api/v1/compat/profiles')
			.then((data) => (compat = data))
			.catch((err) => (error = errorText(err instanceof ApiError ? err.code : 'unknown')));
	});

	const live = $derived(systemStatus.link === 'live');
	function deviceState(profile: string) {
		return systemStatus.data?.devices.find((d) => d.profile === profile);
	}
</script>

<p class="mb-5 text-sm text-muted">{t('devices.subtitle')}</p>
<FormError message={error} />

<p
	class="mb-5 rounded-lg border border-warn/30 bg-warn-soft px-4 py-2.5 text-[13px] text-warn"
	role="note"
>
	{t('devices.noNative')}
</p>

<div class="flex flex-col gap-6">
	{#if compat === null && !error}
		{#each [0, 1] as i (i)}
			<div class="h-40 animate-pulse rounded-xl bg-surface" aria-hidden="true"></div>
		{/each}
	{/if}
	{#each compat?.profiles ?? [] as entry (entry.profile)}
		{@const device = deviceState(entry.profile)}
		<section
			class="rounded-xl border border-line bg-surface shadow-card"
			aria-labelledby="p-{entry.profile}"
		>
			<div class="flex flex-wrap items-center gap-3 border-b border-line px-4 py-3">
				<h2 id="p-{entry.profile}" class="text-sm font-semibold">
					{t(`device.${entry.profile}` as MessageKey)}
				</h2>
				<StatusChip
					tone={deviceTone(live ? (device?.state ?? null) : null)}
					label={live
						? t(`device.state.${device?.state ?? 'null'}` as MessageKey)
						: t('status.unknown')}
				/>
				<span class="ml-auto text-xs text-faint">{t('devices.lifecycleNote')}</span>
			</div>
			{#each entry.ranges as range (range.id)}
				<div class="px-4 pt-3 text-xs text-muted">
					<span class="font-medium text-fg">{t('devices.versions')}:</span>
					<span class="font-mono">≥ {range.min} &lt; {range.max_exclusive}</span>
					<span class="mx-1 text-faint">·</span>
					<span>{t('devices.source')}: {range.source}</span>
				</div>
				<div class="relative overflow-x-auto">
					<table class="mt-2 w-full min-w-[560px] text-[13px]">
						<thead class="text-left text-xs text-faint">
							<tr>
								<th scope="col" class="px-4 py-2 font-medium">{t('devices.capability')}</th>
								<th scope="col" class="px-4 py-2 font-medium" title={t('evidence.tooltip')}>
									{t('devices.evidence')}
								</th>
								<th scope="col" class="px-4 py-2 font-medium">{t('devices.note')}</th>
							</tr>
						</thead>
						<tbody>
							{#each range.claims as claim (claim.capability)}
								<tr class="border-t border-line">
									<td class="px-4 py-2.5">{t(`cap.${claim.capability}` as MessageKey)}</td>
									<td class="px-4 py-2.5">
										<StatusChip
											tone={evidenceTone(claim.evidence)}
											label={t(`evidence.${claim.evidence}` as MessageKey)}
										/>
									</td>
									<td class="px-4 py-2.5 text-muted">{claim.note || '—'}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/each}
		</section>
	{/each}
</div>
