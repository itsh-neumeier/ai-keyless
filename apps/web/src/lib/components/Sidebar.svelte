<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { NAV, findNav, type NavItem } from '#lib/nav.ts';
	import Icon from './Icon.svelte';

	let {
		collapsed,
		mobileOpen,
		onToggle,
		onNavigate
	}: {
		collapsed: boolean;
		mobileOpen: boolean;
		onToggle: () => void;
		onNavigate: () => void;
	} = $props();

	function hrefOf(item: NavItem) {
		return item.href === '/'
			? resolve('/')
			: item.implemented
				? resolve(item.href as '/system' | '/settings')
				: resolve('/[section]', { section: item.href.slice(1) });
	}

	const active = $derived(findNav(page.url.pathname)?.id);
	const groups = [NAV.filter((i) => i.group === 'main'), NAV.filter((i) => i.group === 'admin')];
</script>

<nav
	aria-label={t('nav.main')}
	class="fixed inset-y-0 left-0 z-40 flex flex-col border-r border-line bg-surface transition-[width,transform] duration-150
		{collapsed ? 'md:w-14' : 'md:w-52'} w-60
		{mobileOpen ? 'translate-x-0' : '-translate-x-full'} md:translate-x-0"
>
	<div class="flex h-12 shrink-0 items-center gap-2 border-b border-line px-3.5">
		<img src="/favicon.svg" alt="" class="size-6 shrink-0" />
		{#if !collapsed}
			<span class="truncate text-sm font-semibold tracking-tight">{t('app.name')}</span>
		{/if}
	</div>

	<div class="flex flex-1 flex-col justify-between overflow-y-auto py-2">
		{#each groups as group, gi (gi)}
			<ul
				class="flex flex-col gap-0.5 px-2"
				class:border-t={gi > 0}
				class:border-line={gi > 0}
				class:pt-2={gi > 0}
			>
				{#each group as item (item.id)}
					<li>
						<a
							href={hrefOf(item)}
							onclick={onNavigate}
							aria-current={active === item.id ? 'page' : undefined}
							title={collapsed ? t(item.label) : undefined}
							class="flex h-9 items-center gap-3 rounded-md px-2.5 text-[13px] transition-colors
								{active === item.id
								? 'bg-accent-soft text-accent'
								: 'text-muted hover:bg-surface-hover hover:text-fg'}"
						>
							<span class="shrink-0"><Icon name={item.icon} /></span>
							<span class="truncate {collapsed ? 'md:sr-only' : ''}">{t(item.label)}</span>
							{#if !item.implemented && !collapsed}
								<span
									class="ml-auto size-1.5 shrink-0 rounded-full bg-line-strong"
									aria-hidden="true"
								></span>
							{/if}
						</a>
					</li>
				{/each}
			</ul>
		{/each}
	</div>

	<button
		type="button"
		onclick={onToggle}
		class="hidden h-10 shrink-0 items-center gap-3 border-t border-line px-4.5 text-muted hover:text-fg md:flex"
		aria-label={collapsed ? t('nav.expand') : t('nav.collapse')}
		aria-expanded={!collapsed}
	>
		<Icon name={collapsed ? 'chevronRight' : 'chevronLeft'} size={16} />
	</button>
</nav>
