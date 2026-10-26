<script lang="ts">
	import type { Snippet } from 'svelte';
	import Button from './Button.svelte';

	// Zweistufige Bestätigung für folgenreiche Aktionen (bis Dialog-Komponente in M020-03).
	let {
		confirmLabel,
		onconfirm,
		busy = false,
		variant = 'secondary',
		children
	}: {
		confirmLabel: string;
		onconfirm: () => void;
		busy?: boolean;
		variant?: 'primary' | 'secondary' | 'ghost';
		children: Snippet;
	} = $props();

	let armed = $state(false);
</script>

{#if armed}
	<span class="inline-flex items-center gap-2">
		<Button
			variant="primary"
			{busy}
			onclick={() => {
				armed = false;
				onconfirm();
			}}>{confirmLabel}</Button
		>
		<Button variant="ghost" onclick={() => (armed = false)}>✕</Button>
	</span>
{:else}
	<Button {variant} {busy} onclick={() => (armed = true)}>{@render children()}</Button>
{/if}
