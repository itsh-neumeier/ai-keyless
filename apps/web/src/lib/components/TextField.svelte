<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	let {
		id,
		label,
		hint,
		value = $bindable(''),
		...rest
	}: { id: string; label: string; hint?: string; value?: string } & Omit<
		HTMLInputAttributes,
		'value' | 'id'
	> = $props();
</script>

<div class="flex flex-col gap-1">
	<label for={id} class="text-xs font-medium text-muted">{label}</label>
	<input
		{id}
		bind:value
		aria-describedby={hint ? `${id}-hint` : undefined}
		class="h-9 rounded-md border border-line bg-surface-2 px-3 text-[13px] text-fg placeholder:text-faint focus:border-accent focus:outline-none"
		{...rest}
	/>
	{#if hint}<p id="{id}-hint" class="text-[11px] text-faint">{hint}</p>{/if}
</div>
