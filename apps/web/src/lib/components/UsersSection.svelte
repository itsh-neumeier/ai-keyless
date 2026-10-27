<script lang="ts">
	import { ApiError, api } from '#lib/api/client.ts';
	import type { Role, UserView } from '#lib/auth/auth.svelte.ts';
	import { errorText, t, type MessageKey } from '#lib/i18n/index.svelte.ts';
	import Button from './Button.svelte';
	import FormError from './FormError.svelte';
	import StatusChip from './StatusChip.svelte';
	import TextField from './TextField.svelte';

	const ROLES: Role[] = ['viewer', 'operator', 'admin'];

	let users = $state<UserView[] | null>(null);
	let loadError = $state<string | null>(null);
	let username = $state('');
	let password = $state('');
	let role = $state<Role>('viewer');
	let busy = $state(false);
	let formError = $state<string | null>(null);
	let notice = $state<string | null>(null);

	async function load() {
		try {
			users = await api<UserView[]>('/api/v1/users');
			loadError = null;
		} catch (err) {
			loadError = errorText(err instanceof ApiError ? err.code : 'unknown');
		}
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		formError = notice = null;
		try {
			await api('/api/v1/users', { method: 'POST', body: { username, password, role } });
			username = password = '';
			role = 'viewer';
			notice = t('users.created');
			await load();
		} catch (err) {
			formError = errorText(err instanceof ApiError ? err.code : 'unknown');
		} finally {
			busy = false;
		}
	}

	$effect(() => {
		load();
	});
</script>

<section
	class="max-w-2xl rounded-xl border border-line bg-surface shadow-card"
	aria-labelledby="users"
>
	<div class="border-b border-line px-4 py-3">
		<h2 id="users" class="text-sm font-semibold">{t('users.title')}</h2>
	</div>
	<FormError message={loadError} />
	<div class="relative overflow-x-auto">
		<table class="w-full min-w-[420px] text-[13px]">
			<thead class="text-left text-xs text-faint">
				<tr>
					<th scope="col" class="px-4 py-2 font-medium">{t('auth.username')}</th>
					<th scope="col" class="px-4 py-2 font-medium">{t('users.role')}</th>
					<th scope="col" class="px-4 py-2 font-medium">{t('users.status')}</th>
				</tr>
			</thead>
			<tbody>
				{#if users === null && !loadError}
					<tr class="border-t border-line">
						<td colspan="3" class="px-4 py-3">
							<div class="h-4 w-1/2 animate-pulse rounded bg-surface-2" aria-hidden="true"></div>
						</td>
					</tr>
				{/if}
				{#each users ?? [] as user (user.id)}
					<tr class="border-t border-line">
						<td class="px-4 py-2.5 font-medium">{user.username}</td>
						<td class="px-4 py-2.5">{t(`role.${user.role}` as MessageKey)}</td>
						<td class="px-4 py-2.5">
							<StatusChip
								tone={user.disabled ? 'neutral' : 'ok'}
								label={user.disabled ? t('users.disabled') : t('users.active')}
							/>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
	<form class="grid gap-3 border-t border-line px-4 py-4 sm:grid-cols-3" onsubmit={create}>
		<TextField
			id="new-username"
			label={t('auth.username')}
			bind:value={username}
			autocomplete="off"
			spellcheck={false}
			required
		/>
		<TextField
			id="new-password"
			type="password"
			label={t('auth.password')}
			hint={t('auth.passwordHint')}
			bind:value={password}
			autocomplete="new-password"
			required
		/>
		<div class="flex flex-col gap-1">
			<label for="new-role" class="text-xs font-medium text-muted">{t('users.role')}</label>
			<select
				id="new-role"
				bind:value={role}
				class="h-9 rounded-md border border-line bg-surface-2 px-2 text-[13px]"
			>
				{#each ROLES as r (r)}
					<option value={r}>{t(`role.${r}` as MessageKey)}</option>
				{/each}
			</select>
		</div>
		<div class="flex flex-col gap-2 sm:col-span-3">
			<FormError message={formError} />
			{#if notice}<p class="text-[13px] text-ok" role="status">{notice}</p>{/if}
			<div><Button type="submit" {busy}>{t('users.add')}</Button></div>
		</div>
	</form>
</section>
