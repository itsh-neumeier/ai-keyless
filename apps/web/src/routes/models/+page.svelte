<script lang="ts">
	import { ApiError, api } from '#lib/api/client.ts';
	import type {
		ConnectionResult,
		ProviderKind,
		ProviderTest,
		ProviderView
	} from '#lib/api/providers.ts';
	import { auth } from '#lib/auth/auth.svelte.ts';
	import Button from '#lib/components/Button.svelte';
	import ConfirmButton from '#lib/components/ConfirmButton.svelte';
	import FormError from '#lib/components/FormError.svelte';
	import StatusChip from '#lib/components/StatusChip.svelte';
	import TextField from '#lib/components/TextField.svelte';
	import { errorText, i18n, t, type MessageKey } from '#lib/i18n/index.svelte.ts';

	const KINDS: ProviderKind[] = ['ollama', 'openai_compatible'];

	let providers = $state<ProviderView[] | null>(null);
	let loadError = $state<string | null>(null);
	/** Modelle aus dem letzten Verbindungstest dieser Sitzung (für die Auswahl). */
	let models = $state<Record<string, string[]>>({});
	let busy = $state<Record<string, string | null>>({});
	let rowError = $state<Record<string, string | null>>({});
	/** Für den Bildtest gewähltes Modell; leer = Standardmodell. */
	let visionModel = $state<Record<string, string>>({});

	let name = $state('');
	let kind = $state<ProviderKind>('ollama');
	let baseUrl = $state('');
	let secret = $state('');
	let defaultModel = $state('');
	let creating = $state(false);
	let formError = $state<string | null>(null);

	const isAdmin = $derived(auth.can('admin'));

	function code(err: unknown) {
		return errorText(err instanceof ApiError ? err.code : 'unknown');
	}

	async function load() {
		try {
			providers = await api<ProviderView[]>('/api/v1/providers');
			loadError = null;
		} catch (err) {
			loadError = code(err);
		}
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		creating = true;
		formError = null;
		try {
			await api('/api/v1/providers', {
				method: 'POST',
				body: {
					name,
					kind,
					base_url: baseUrl,
					api_key_secret: secret || null,
					default_model: defaultModel || null
				}
			});
			name = baseUrl = secret = defaultModel = '';
			await load();
		} catch (err) {
			formError = code(err);
		} finally {
			creating = false;
		}
	}

	async function run(id: string, action: string, fn: () => Promise<void>) {
		busy[id] = action;
		rowError[id] = null;
		try {
			await fn();
			await load();
		} catch (err) {
			rowError[id] = code(err);
		} finally {
			busy[id] = null;
		}
	}

	const testConnection = (p: ProviderView) =>
		run(p.id, 'connection', async () => {
			const r = await api<ConnectionResult>(`/api/v1/providers/${p.id}/test`, { method: 'POST' });
			models[p.id] = r.models;
		});
	const testVision = (p: ProviderView) =>
		run(p.id, 'vision', async () => {
			const model = visionModel[p.id] || null;
			await api(`/api/v1/providers/${p.id}/test-vision`, { method: 'POST', body: { model } });
		});
	let notice = $state<Record<string, string | null>>({});
	const enqueueProbe = (p: ProviderView) =>
		run(p.id, 'probe', async () => {
			const model = visionModel[p.id] || null;
			await api(`/api/v1/providers/${p.id}/probe-job`, { method: 'POST', body: { model } });
			notice[p.id] = t('models.enqueued');
		});
	const remove = (p: ProviderView) =>
		run(p.id, 'delete', async () => {
			await api(`/api/v1/providers/${p.id}`, { method: 'DELETE' });
		});

	function latest(p: ProviderView, kind: ProviderTest['kind']) {
		return p.tests.find((x) => x.kind === kind);
	}

	function when(iso: string) {
		return new Date(iso).toLocaleString(i18n.locale === 'de' ? 'de-DE' : 'en-GB');
	}

	$effect(() => {
		load();
	});
</script>

<p class="mb-5 text-sm text-muted">{t('models.subtitle')}</p>
<FormError message={loadError} />

<div class="flex flex-col gap-4">
	{#if providers === null && !loadError}
		<div class="h-32 animate-pulse rounded-xl bg-surface" aria-hidden="true"></div>
	{:else if providers?.length === 0}
		<p
			class="rounded-xl border border-dashed border-line-strong px-4 py-8 text-center text-sm text-muted"
		>
			{t('models.empty')}
		</p>
	{/if}

	{#each providers ?? [] as p (p.id)}
		{@const conn = latest(p, 'connection')}
		{@const vision = latest(p, 'vision')}
		<section
			class="rounded-xl border border-line bg-surface shadow-card"
			aria-labelledby="prov-{p.id}"
		>
			<div class="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-line px-4 py-3">
				<h2 id="prov-{p.id}" class="text-sm font-semibold">{p.name}</h2>
				<span class="text-xs text-muted">{t(`models.kind.${p.kind}` as MessageKey)}</span>
				<span class="font-mono text-xs break-all text-faint">{p.base_url}</span>
			</div>
			<dl class="grid gap-x-6 gap-y-3 px-4 py-3 text-[13px] sm:grid-cols-2">
				<div>
					<dt class="mb-1 text-xs text-muted">{t('models.connection')}</dt>
					<dd class="flex flex-wrap items-center gap-2">
						{#if conn}
							<StatusChip
								tone={conn.ok ? 'ok' : 'danger'}
								label={conn.ok ? t('models.ok') : t('models.failed')}
							/>
							{#if conn.latency_ms !== null}
								<span class="font-mono text-xs text-muted tabular-nums">{conn.latency_ms} ms</span>
							{/if}
							{#if conn.error_code}<span class="text-xs text-danger"
									>{errorText(conn.error_code)}</span
								>{/if}
							<span class="text-xs text-faint">{when(conn.tested_at)}</span>
						{:else}
							<StatusChip label={t('models.notTested')} />
						{/if}
					</dd>
				</div>
				<div>
					<dt class="mb-1 text-xs text-muted">
						{t('models.vision')}
						{#if vision?.model}<span class="ml-1 font-mono">· {vision.model}</span>{/if}
					</dt>
					<dd class="flex flex-wrap items-center gap-2">
						{#if vision}
							{#if vision.ok}
								<StatusChip
									tone={vision.matched ? 'ok' : 'warn'}
									label={vision.matched ? t('models.matched') : t('models.notMatched')}
								/>
								<span class="font-mono text-xs text-muted tabular-nums">{vision.latency_ms} ms</span
								>
								<span class="text-xs text-muted">{t('models.answer')}: „{vision.result_text}“</span>
							{:else}
								<StatusChip tone="danger" label={t('models.failed')} />
								<span class="text-xs text-danger">{errorText(vision.error_code ?? 'unknown')}</span>
							{/if}
							<span class="text-xs text-faint">{when(vision.tested_at)}</span>
						{:else}
							<StatusChip label={t('models.notTested')} />
						{/if}
					</dd>
				</div>
				{#if models[p.id]?.length}
					<div class="sm:col-span-2">
						<dt class="mb-1 text-xs text-muted">{t('models.models')}</dt>
						<dd class="flex flex-wrap gap-1.5">
							{#each models[p.id] ?? [] as m (m)}
								<span class="rounded bg-surface-2 px-1.5 py-0.5 font-mono text-xs">{m}</span>
							{/each}
						</dd>
					</div>
				{/if}
				<div class="text-xs text-muted sm:col-span-2">
					{t('models.defaultModel')}:
					<span class="font-mono">{p.default_model ?? '—'}</span>
					{#if p.api_key_secret}
						<span class="mx-1 text-faint">·</span>{t('models.secret')}:
						<span class="font-mono">{p.api_key_secret}</span>
					{/if}
				</div>
			</dl>
			{#if isAdmin}
				<div class="flex flex-wrap items-center gap-2 border-t border-line px-4 py-3">
					<Button
						variant="secondary"
						busy={busy[p.id] === 'connection'}
						onclick={() => testConnection(p)}>{t('models.testConnection')}</Button
					>
					{#if models[p.id]?.length}
						<label class="sr-only" for="vm-{p.id}">{t('models.visionModel')}</label>
						<select
							id="vm-{p.id}"
							bind:value={visionModel[p.id]}
							class="h-9 rounded-md border border-line bg-surface-2 px-2 font-mono text-[12px]"
						>
							<option value="">{p.default_model ?? '—'} ({t('models.defaultModel')})</option>
							{#each models[p.id] ?? [] as m (m)}
								<option value={m}>{m}</option>
							{/each}
						</select>
					{/if}
					<ConfirmButton
						confirmLabel={t('models.testVisionConfirm')}
						busy={busy[p.id] === 'vision'}
						onconfirm={() => testVision(p)}>{t('models.testVision')}</ConfirmButton
					>
					<Button variant="ghost" busy={busy[p.id] === 'probe'} onclick={() => enqueueProbe(p)}
						>{t('models.enqueueProbe')}</Button
					>
					<span class="ml-auto"></span>
					<ConfirmButton
						variant="ghost"
						confirmLabel={t('models.deleteConfirm')}
						busy={busy[p.id] === 'delete'}
						onconfirm={() => remove(p)}>{t('models.delete')}</ConfirmButton
					>
				</div>
				<div class="px-4 pb-3 empty:hidden">
					<FormError message={rowError[p.id] ?? null} />
					{#if notice[p.id]}<p class="text-[13px] text-ok" role="status">{notice[p.id]}</p>{/if}
				</div>
			{/if}
		</section>
	{/each}
</div>

{#if isAdmin}
	<p class="mt-3 text-xs text-faint">{t('models.visionHint')}</p>

	<section
		class="mt-6 rounded-xl border border-line bg-surface shadow-card"
		aria-labelledby="add-provider"
	>
		<div class="border-b border-line px-4 py-3">
			<h2 id="add-provider" class="text-sm font-semibold">{t('models.add')}</h2>
			<p class="mt-0.5 text-xs text-faint">{t('models.cloudNote')}</p>
		</div>
		<form class="grid gap-4 px-4 py-4 sm:grid-cols-2" onsubmit={create}>
			<TextField id="p-name" label={t('models.name')} bind:value={name} required />
			<div class="flex flex-col gap-1">
				<label for="p-kind" class="text-xs font-medium text-muted">{t('models.kind')}</label>
				<select
					id="p-kind"
					bind:value={kind}
					class="h-9 rounded-md border border-line bg-surface-2 px-2 text-[13px]"
				>
					{#each KINDS as k (k)}
						<option value={k}>{t(`models.kind.${k}` as MessageKey)}</option>
					{/each}
				</select>
			</div>
			<div class="sm:col-span-2">
				<TextField
					id="p-url"
					label={t('models.url')}
					hint={t('models.urlHint')}
					bind:value={baseUrl}
					type="url"
					spellcheck={false}
					required
				/>
			</div>
			<TextField
				id="p-secret"
				label={t('models.secret')}
				hint={t('models.secretHint')}
				bind:value={secret}
				spellcheck={false}
			/>
			<TextField
				id="p-model"
				label={t('models.defaultModel')}
				bind:value={defaultModel}
				spellcheck={false}
			/>
			<div class="flex flex-col gap-2 sm:col-span-2">
				<FormError message={formError} />
				<div><Button type="submit" busy={creating}>{t('models.add')}</Button></div>
			</div>
		</form>
	</section>
{/if}
