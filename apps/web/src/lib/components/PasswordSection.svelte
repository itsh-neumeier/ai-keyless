<script lang="ts">
	import { ApiError, api } from '#lib/api/client.ts';
	import { errorText, t } from '#lib/i18n/index.svelte.ts';
	import Button from './Button.svelte';
	import FormError from './FormError.svelte';
	import TextField from './TextField.svelte';

	let current = $state('');
	let next = $state('');
	let repeat = $state('');
	let busy = $state(false);
	let error = $state<string | null>(null);
	let notice = $state<string | null>(null);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = notice = null;
		if (next !== repeat) {
			error = t('auth.passwordMismatch');
			return;
		}
		busy = true;
		try {
			await api('/api/v1/auth/password', {
				method: 'POST',
				body: { current_password: current, new_password: next }
			});
			current = next = repeat = '';
			notice = t('password.changed');
		} catch (err) {
			error = errorText(err instanceof ApiError ? err.code : 'unknown');
		} finally {
			busy = false;
		}
	}
</script>

<section
	class="max-w-2xl rounded-xl border border-line bg-surface shadow-card"
	aria-labelledby="password"
>
	<div class="border-b border-line px-4 py-3">
		<h2 id="password" class="text-sm font-semibold">{t('password.title')}</h2>
		<p class="mt-0.5 text-xs text-faint">{t('password.hint')}</p>
	</div>
	<form class="grid gap-3 px-4 py-4 sm:grid-cols-3" onsubmit={submit}>
		<TextField
			id="pw-current"
			type="password"
			label={t('password.current')}
			bind:value={current}
			autocomplete="current-password"
			required
		/>
		<TextField
			id="pw-new"
			type="password"
			label={t('password.new')}
			hint={t('auth.passwordHint')}
			bind:value={next}
			autocomplete="new-password"
			required
		/>
		<TextField
			id="pw-repeat"
			type="password"
			label={t('auth.passwordRepeat')}
			bind:value={repeat}
			autocomplete="new-password"
			required
		/>
		<div class="flex flex-col gap-2 sm:col-span-3">
			<FormError message={error} />
			{#if notice}<p class="text-[13px] text-ok" role="status">{notice}</p>{/if}
			<div><Button type="submit" {busy}>{t('password.submit')}</Button></div>
		</div>
	</form>
</section>
