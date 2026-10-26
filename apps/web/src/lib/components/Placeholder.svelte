<script lang="ts">
	import { t } from '#lib/i18n/index.svelte.ts';
	import type { NavItem } from '#lib/nav.ts';
	import Icon from './Icon.svelte';

	let { item }: { item: NavItem } = $props();
</script>

<section
	class="mx-auto mt-10 flex max-w-lg flex-col items-center rounded-xl border border-dashed border-line-strong px-6 py-12 text-center"
>
	<span class="mb-4 grid size-12 place-items-center rounded-full bg-surface-2 text-muted">
		<Icon name={item.icon} size={22} />
	</span>
	<h2 class="text-base font-semibold">{t('placeholder.title')}</h2>
	<p class="mt-2 text-sm text-muted">{t('placeholder.body')}</p>
	<p class="mt-4 font-mono text-xs text-faint">{t('placeholder.milestone')}: {item.milestone}</p>
</section>
