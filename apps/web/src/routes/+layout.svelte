<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import { systemStatus } from '#lib/api/status.svelte.ts';
	import { auth } from '#lib/auth/auth.svelte.ts';
	import AuthScreen from '#lib/components/AuthScreen.svelte';
	import Sidebar from '#lib/components/Sidebar.svelte';
	import Topbar from '#lib/components/Topbar.svelte';
	import { i18n, t } from '#lib/i18n/index.svelte.ts';
	import { findNav } from '#lib/nav.ts';
	import { theme } from '#lib/stores/theme.svelte.ts';

	let { children } = $props();

	const COLLAPSE_KEY = 'akl.nav.collapsed';
	let collapsed = $state(readCollapsed());
	let mobileOpen = $state(false);

	function readCollapsed(): boolean {
		try {
			return localStorage.getItem(COLLAPSE_KEY) === '1';
		} catch {
			return false;
		}
	}

	function toggle() {
		collapsed = !collapsed;
		try {
			localStorage.setItem(COLLAPSE_KEY, collapsed ? '1' : '0');
		} catch {
			/* nur Komfort */
		}
	}

	const current = $derived(findNav(page.url.pathname));
	const title = $derived(current ? t(current.label) : t('common.notFound'));

	$effect(() => {
		auth.refresh();
	});
	// Status-Stream nur mit gültiger Sitzung öffnen.
	$effect(() => {
		if (auth.phase === 'ready') return systemStatus.connect();
	});
	// Bricht der Stream ab (Abmeldung, Ablauf, Netz), Anmeldezustand neu prüfen.
	$effect(() => {
		if (auth.phase === 'ready' && systemStatus.link === 'offline') auth.refresh();
	});
	$effect(() => {
		document.documentElement.lang = i18n.locale;
	});
	$effect(() => {
		const mq = matchMedia('(prefers-color-scheme: light)');
		const onChange = () => theme.apply();
		mq.addEventListener('change', onChange);
		return () => mq.removeEventListener('change', onChange);
	});
</script>

<svelte:head>
	<title>{title} · AI-Keyless</title>
</svelte:head>

{#if auth.phase !== 'ready'}
	<AuthScreen />
{:else}
	<a
		href="#main"
		class="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-50 focus:rounded focus:bg-surface focus:px-3 focus:py-2"
		>{t('common.skipToContent')}</a
	>

	<Sidebar {collapsed} {mobileOpen} onToggle={toggle} onNavigate={() => (mobileOpen = false)} />

	{#if mobileOpen}
		<button
			type="button"
			class="fixed inset-0 z-30 bg-black/40 md:hidden"
			aria-label={t('nav.collapse')}
			onclick={() => (mobileOpen = false)}
		></button>
	{/if}

	<div class="min-h-dvh transition-[padding] duration-150 {collapsed ? 'md:pl-14' : 'md:pl-52'}">
		<Topbar {title} onMenu={() => (mobileOpen = true)} />
		<main id="main" class="mx-auto max-w-7xl px-4 py-5 sm:px-6">
			{@render children()}
		</main>
	</div>
{/if}
