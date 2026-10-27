<script lang="ts">
	import { ApiError, api } from '#lib/api/client.ts';
	import {
		deviceSignature,
		type DeviceKind,
		type DeviceList,
		type DeviceProfile,
		type Transition
	} from '#lib/api/devices.ts';
	import { systemStatus } from '#lib/api/status.svelte.ts';
	import { auth } from '#lib/auth/auth.svelte.ts';
	import Button from './Button.svelte';
	import ConfirmButton from './ConfirmButton.svelte';
	import FormError from './FormError.svelte';
	import StatusChip from './StatusChip.svelte';
	import TextField from './TextField.svelte';
	import { deviceTone } from './tone.ts';
	import { errorText, i18n, t, type MessageKey } from '#lib/i18n/index.svelte.ts';

	const KINDS: DeviceKind[] = ['ai_key', 'ai_port'];

	let list = $state<DeviceList | null>(null);
	let loadError = $state<string | null>(null);
	let busy = $state<Record<string, string | null>>({});
	let rowError = $state<Record<string, string | null>>({});
	let history = $state<Record<string, Transition[] | null>>({});

	let kind = $state<DeviceKind>('ai_key');
	let name = $state('');
	let creating = $state(false);
	let formError = $state<string | null>(null);
	let killBusy = $state(false);
	let killError = $state<string | null>(null);

	const isAdmin = $derived(auth.can('admin'));
	const killActive = $derived(list?.kill_switch.active ?? false);

	function code(err: unknown) {
		return errorText(err instanceof ApiError ? err.code : 'unknown');
	}

	function when(iso: string | null) {
		return iso ? new Date(iso).toLocaleString(i18n.locale === 'de' ? 'de-DE' : 'en-GB') : '—';
	}

	async function load() {
		try {
			list = await api<DeviceList>('/api/v1/devices');
			loadError = null;
			// Geöffnete Verläufe aktuell halten.
			for (const id of Object.keys(history)) if (history[id]) await loadHistory(id);
		} catch (err) {
			loadError = code(err);
		}
	}

	async function loadHistory(id: string) {
		const r = await api<{ transitions: Transition[] }>(`/api/v1/devices/${id}/transitions`);
		history[id] = r.transitions;
	}

	async function toggleHistory(p: DeviceProfile) {
		if (history[p.id]) {
			history[p.id] = null;
			return;
		}
		await run(p.id, 'history', () => loadHistory(p.id), false);
	}

	async function run(id: string, action: string, fn: () => Promise<void>, reload = true) {
		busy[id] = action;
		rowError[id] = null;
		try {
			await fn();
			if (reload) await load();
		} catch (err) {
			rowError[id] = code(err);
		} finally {
			busy[id] = null;
		}
	}

	const disable = (p: DeviceProfile) =>
		run(p.id, 'disable', async () => {
			await api(`/api/v1/devices/${p.id}/disable`, { method: 'POST' });
		});
	const enable = (p: DeviceProfile) =>
		run(p.id, 'enable', async () => {
			await api(`/api/v1/devices/${p.id}/enable`, { method: 'POST' });
		});

	async function create(event: SubmitEvent) {
		event.preventDefault();
		creating = true;
		formError = null;
		try {
			await api('/api/v1/devices', { method: 'POST', body: { kind, display_name: name } });
			name = '';
			await load();
		} catch (err) {
			formError = code(err);
		} finally {
			creating = false;
		}
	}

	async function setKill(active: boolean) {
		killBusy = true;
		killError = null;
		try {
			await api('/api/v1/devices/kill-switch', { method: 'PUT', body: { active } });
			await load();
		} catch (err) {
			killError = code(err);
		} finally {
			killBusy = false;
		}
	}

	// Echtzeit: der SSE-Status meldet geänderte Gerätezustände → neu laden.
	const signature = $derived(deviceSignature(systemStatus.data?.devices));
	$effect(() => {
		void signature;
		load();
	});
</script>

<section
	class="rounded-xl border border-line bg-surface shadow-card"
	aria-labelledby="dev-profiles"
>
	<div class="flex flex-wrap items-center gap-3 border-b border-line px-4 py-3">
		<h2 id="dev-profiles" class="text-sm font-semibold">{t('devices.profiles')}</h2>
		{#if list}
			<StatusChip
				tone={killActive ? 'danger' : 'ok'}
				label={killActive ? t('devices.killOn') : t('devices.killOff')}
			/>
			{#if list.kill_switch.forced}
				<span class="text-xs text-danger">{t('devices.killForced')}</span>
			{:else if list.kill_switch.changed_at}
				<span class="text-xs text-faint"
					>{list.kill_switch.changed_by ?? '—'} · {when(list.kill_switch.changed_at)}</span
				>
			{/if}
		{/if}
		{#if isAdmin && list && !list.kill_switch.forced}
			<span class="ml-auto">
				{#if killActive}
					<ConfirmButton
						variant="secondary"
						confirmLabel={t('devices.killReleaseConfirm')}
						busy={killBusy}
						onconfirm={() => setKill(false)}>{t('devices.killRelease')}</ConfirmButton
					>
				{:else}
					<ConfirmButton
						variant="ghost"
						confirmLabel={t('devices.killEngageConfirm')}
						busy={killBusy}
						onconfirm={() => setKill(true)}>{t('devices.killEngage')}</ConfirmButton
					>
				{/if}
			</span>
		{/if}
	</div>
	<div class="px-4 empty:hidden"><FormError message={killError} /></div>

	<div class="px-4 py-3">
		<FormError message={loadError} />
		{#if list === null && !loadError}
			<div class="h-16 animate-pulse rounded-lg bg-surface-2" aria-hidden="true"></div>
		{:else if list?.profiles.length === 0}
			<p class="py-4 text-center text-sm text-muted">{t('devices.noProfiles')}</p>
		{/if}

		<ul class="flex flex-col divide-y divide-line">
			{#each list?.profiles ?? [] as p (p.id)}
				<li class="py-3">
					<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
						<span class="text-sm font-medium">{p.display_name}</span>
						<span class="text-xs text-muted">{t(`device.${p.kind}` as MessageKey)}</span>
						<StatusChip
							tone={deviceTone(p.state)}
							label={t(`device.state.${p.state}` as MessageKey)}
						/>
						<span class="text-xs text-faint">{t('devices.since')} {when(p.updated_at)}</span>
					</div>
					<p class="mt-1 text-xs text-muted">
						{t('devices.fingerprint')}:
						<span class="font-mono break-all">{p.fingerprint ?? '—'}</span>
					</p>
					<div class="mt-2 flex flex-wrap items-center gap-2">
						<Button variant="ghost" busy={busy[p.id] === 'history'} onclick={() => toggleHistory(p)}
							>{history[p.id] ? t('devices.hideHistory') : t('devices.showHistory')}</Button
						>
						{#if isAdmin}
							{#if p.state === 'disabled'}
								<Button variant="secondary" busy={busy[p.id] === 'enable'} onclick={() => enable(p)}
									>{t('devices.enable')}</Button
								>
							{:else}
								<ConfirmButton
									variant="secondary"
									confirmLabel={t('devices.disableConfirm')}
									busy={busy[p.id] === 'disable'}
									onconfirm={() => disable(p)}>{t('devices.disable')}</ConfirmButton
								>
							{/if}
						{/if}
					</div>
					<FormError message={rowError[p.id] ?? null} />
					{#if history[p.id]}
						<div class="relative mt-2 overflow-x-auto">
							<table class="w-full min-w-[520px] text-[12px]">
								<thead class="text-left text-faint">
									<tr>
										<th scope="col" class="py-1 pr-3 font-medium">{t('devices.when')}</th>
										<th scope="col" class="py-1 pr-3 font-medium">{t('devices.transition')}</th>
										<th scope="col" class="py-1 pr-3 font-medium">{t('devices.reason')}</th>
										<th scope="col" class="py-1 font-medium">{t('devices.actor')}</th>
									</tr>
								</thead>
								<tbody>
									{#each history[p.id] ?? [] as tr (tr.id)}
										<tr class="border-t border-line">
											<td class="py-1 pr-3 whitespace-nowrap text-muted">{when(tr.occurred_at)}</td>
											<td class="py-1 pr-3">
												{t(`device.state.${tr.from_state}` as MessageKey)} → {t(
													`device.state.${tr.to_state}` as MessageKey
												)}
											</td>
											<td class="py-1 pr-3 text-muted">{tr.reason ?? '—'}</td>
											<td class="py-1 font-mono text-muted">{tr.actor}</td>
										</tr>
									{:else}
										<tr><td colspan="4" class="py-2 text-muted">{t('devices.noHistory')}</td></tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	</div>

	{#if isAdmin && list}
		<form
			class="flex flex-wrap items-end gap-3 border-t border-line px-4 py-3"
			onsubmit={create}
			aria-labelledby="dev-create"
		>
			<h3 id="dev-create" class="sr-only">{t('devices.create')}</h3>
			<div class="flex flex-col gap-1">
				<label for="dev-kind" class="text-xs text-muted">{t('devices.kind')}</label>
				<select
					id="dev-kind"
					bind:value={kind}
					class="h-9 rounded-md border border-line bg-surface-2 px-2 text-[13px]"
				>
					{#each KINDS as k (k)}
						<option value={k}>{t(`device.${k}` as MessageKey)}</option>
					{/each}
				</select>
			</div>
			<div class="min-w-48 flex-1">
				<TextField id="dev-name" label={t('devices.name')} bind:value={name} maxlength={64} />
			</div>
			<Button type="submit" busy={creating} disabled={killActive}>{t('devices.create')}</Button>
			<div class="w-full empty:hidden"><FormError message={formError} /></div>
			<p class="w-full text-xs text-faint">{t('devices.createHint')}</p>
		</form>
	{/if}
</section>
