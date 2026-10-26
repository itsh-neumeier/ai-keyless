<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	let {
		variant = 'primary',
		busy = false,
		children,
		...rest
	}: {
		variant?: 'primary' | 'secondary' | 'ghost';
		busy?: boolean;
		children: Snippet;
	} & HTMLButtonAttributes = $props();

	const styles = {
		primary: 'bg-accent text-white hover:brightness-110',
		secondary: 'border border-line bg-surface-2 text-fg hover:bg-surface-hover',
		ghost: 'text-muted hover:bg-surface-hover hover:text-fg'
	};
</script>

<button
	class="inline-flex h-9 items-center justify-center gap-2 rounded-md px-3.5 text-[13px] font-medium transition disabled:cursor-not-allowed disabled:opacity-60 {styles[
		variant
	]}"
	disabled={busy || rest.disabled}
	aria-busy={busy}
	{...rest}
>
	{#if busy}
		<span
			class="size-3.5 animate-spin rounded-full border-2 border-current border-t-transparent"
			aria-hidden="true"
		></span>
	{/if}
	{@render children()}
</button>
