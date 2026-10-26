<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { t } from '#lib/i18n/index.svelte.ts';
</script>

<section class="mx-auto mt-16 max-w-md text-center">
	<p class="font-mono text-3xl text-faint">{page.status}</p>
	<h2 class="mt-2 text-base font-semibold">{t('common.notFound')}</h2>
	<a href={resolve('/')} class="mt-4 inline-block text-sm text-accent hover:underline"
		>{t('nav.overview')}</a
	>
</section>
