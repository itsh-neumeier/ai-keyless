<script lang="ts">
	import { ApiError } from '#lib/api/client.ts';
	import { auth } from '#lib/auth/auth.svelte.ts';
	import { errorText, t } from '#lib/i18n/index.svelte.ts';
	import Button from './Button.svelte';
	import FormError from './FormError.svelte';
	import TextField from './TextField.svelte';

	let token = $state('');
	let username = $state('');
	let password = $state('');
	let passwordRepeat = $state('');
	let busy = $state(false);
	let error = $state<string | null>(null);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		if (auth.phase === 'setup' && password !== passwordRepeat) {
			error = t('auth.passwordMismatch');
			return;
		}
		busy = true;
		try {
			if (auth.phase === 'setup') await auth.setup(token, username, password);
			else await auth.login(username, password);
			password = passwordRepeat = token = '';
		} catch (err) {
			error = errorText(err instanceof ApiError ? err.code : 'unknown');
			if (err instanceof ApiError && err.code === 'setup_completed') await auth.refresh();
		} finally {
			busy = false;
		}
	}
</script>

<main class="grid min-h-dvh place-items-center bg-bg px-4 py-10">
	<div class="w-full max-w-sm">
		<div class="mb-6 flex items-center justify-center gap-2.5">
			<img src="/favicon.svg" alt="" class="size-8" />
			<span class="text-lg font-semibold tracking-tight">{t('app.name')}</span>
		</div>

		<section class="rounded-xl border border-line bg-surface p-6 shadow-card">
			{#if auth.phase === 'loading'}
				<p class="text-center text-sm text-muted" aria-live="polite">{t('auth.loading')}</p>
			{:else if auth.phase === 'unavailable'}
				<h1 class="text-base font-semibold">{t('auth.unavailableTitle')}</h1>
				<p class="mt-2 text-[13px] text-muted">{errorText(auth.reason ?? 'unknown')}</p>
				<div class="mt-5">
					<Button variant="secondary" onclick={() => auth.refresh()}>{t('auth.retry')}</Button>
				</div>
			{:else}
				<h1 class="text-base font-semibold">
					{auth.phase === 'setup' ? t('auth.setupTitle') : t('auth.loginTitle')}
				</h1>
				{#if auth.phase === 'setup'}
					<p class="mt-2 text-[13px] text-muted">{t('auth.setupIntro')}</p>
					<code
						class="mt-2 block rounded-md bg-surface-2 px-3 py-2 font-mono text-[12px] break-all select-all"
						>{t('auth.setupCommand')}</code
					>
				{/if}
				<form class="mt-5 flex flex-col gap-4" onsubmit={submit} novalidate>
					{#if auth.phase === 'setup'}
						<TextField
							id="token"
							label={t('auth.token')}
							bind:value={token}
							autocomplete="off"
							spellcheck={false}
							required
						/>
					{/if}
					<TextField
						id="username"
						label={t('auth.username')}
						hint={auth.phase === 'setup' ? t('auth.usernameHint') : undefined}
						bind:value={username}
						autocomplete="username"
						autocapitalize="none"
						spellcheck={false}
						required
					/>
					<TextField
						id="password"
						type="password"
						label={t('auth.password')}
						hint={auth.phase === 'setup' ? t('auth.passwordHint') : undefined}
						bind:value={password}
						autocomplete={auth.phase === 'setup' ? 'new-password' : 'current-password'}
						required
					/>
					{#if auth.phase === 'setup'}
						<TextField
							id="password-repeat"
							type="password"
							label={t('auth.passwordRepeat')}
							bind:value={passwordRepeat}
							autocomplete="new-password"
							required
						/>
					{/if}
					<FormError message={error} />
					<Button type="submit" {busy}>
						{auth.phase === 'setup' ? t('auth.createAdmin') : t('auth.login')}
					</Button>
				</form>
			{/if}
		</section>
		<p class="mt-4 text-center text-xs text-faint">{t('app.unofficial')}</p>
	</div>
</main>
