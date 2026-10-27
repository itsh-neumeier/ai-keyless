<script lang="ts">
	import { systemStatus } from '#lib/api/status.svelte.ts';
	import StatusChip from '#lib/components/StatusChip.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
</script>

<section
	class="max-w-2xl rounded-xl border border-line bg-surface shadow-card"
	aria-labelledby="about"
>
	<div class="flex items-center justify-between gap-3 border-b border-line px-4 py-3">
		<h2 id="about" class="text-sm font-semibold">{t('system.about')}</h2>
		<StatusChip tone="warn" label="Independent / Unofficial" />
	</div>
	<div class="space-y-4 px-4 py-4 text-[13px]">
		<p class="text-muted">{t('app.unofficialLong')}</p>
		<dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2">
			<dt class="text-muted">{t('overview.version')}</dt>
			<dd class="font-mono">{systemStatus.data?.version ?? t('common.unknown')}</dd>
			<dt class="text-muted">{t('system.license')}</dt>
			<dd>{t('system.licenseValue')}</dd>
			<dt class="text-muted">{t('system.source')}</dt>
			<dd class="break-all">
				{#if systemStatus.data}
					<!-- Externer Link (Quellcode-Repository), keine App-Route; URL vom Parser auf https geprüft. -->
					<!-- eslint-disable svelte/no-navigation-without-resolve -->
					<a
						href={systemStatus.data.source_url}
						class="text-accent hover:underline"
						target="_blank"
						rel="noopener noreferrer">{systemStatus.data.source_url}</a
					>
					<!-- eslint-enable svelte/no-navigation-without-resolve -->
				{:else}
					{t('common.unknown')}
				{/if}
			</dd>
		</dl>
		<p class="text-xs text-faint">{t('system.sourceNote')}</p>
	</div>
</section>
