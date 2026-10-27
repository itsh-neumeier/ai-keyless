<script lang="ts">
	import { systemStatus } from '#lib/api/status.svelte.ts';
	import { auth } from '#lib/auth/auth.svelte.ts';
	import { t, type MessageKey } from '#lib/i18n/index.svelte.ts';
	import StatusChip from './StatusChip.svelte';
	import { linkTone } from './tone';

	let { title, onMenu }: { title: string; onMenu: () => void } = $props();
</script>

<header
	class="sticky top-0 z-30 flex h-12 items-center gap-3 border-b border-line bg-surface/90 px-4 backdrop-blur"
>
	<button
		type="button"
		class="-ml-1 grid size-8 place-items-center rounded-md text-muted hover:bg-surface-hover md:hidden"
		onclick={onMenu}
		aria-label={t('nav.expand')}
	>
		<svg
			width="18"
			height="18"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16" /></svg
		>
	</button>
	<h1 class="truncate text-sm font-semibold">{title}</h1>
	<div class="ml-auto flex items-center gap-3">
		<span class="hidden text-xs text-faint sm:inline">{t('app.unofficial')}</span>
		<StatusChip
			tone={linkTone(systemStatus.link)}
			label={t(`link.${systemStatus.link}` as MessageKey)}
		/>
		{#if auth.user}
			<span class="hidden h-5 border-l border-line md:inline" aria-hidden="true"></span>
			<span
				class="hidden text-xs text-muted md:inline"
				title={t(`role.${auth.user.role}` as MessageKey)}>{auth.user.username}</span
			>
			<button
				type="button"
				class="rounded-md px-2 py-1 text-xs text-muted hover:bg-surface-hover hover:text-fg"
				onclick={() => auth.logout()}>{t('auth.logout')}</button
			>
		{/if}
	</div>
</header>
