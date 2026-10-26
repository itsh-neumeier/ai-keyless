<script lang="ts">
	import type { Tone } from './tone';

	let { tone = 'neutral', label, title }: { tone?: Tone; label: string; title?: string } = $props();

	const styles: Record<Tone, string> = {
		ok: 'bg-ok-soft text-ok',
		warn: 'bg-warn-soft text-warn',
		danger: 'bg-danger-soft text-danger',
		neutral: 'bg-neutral-soft text-muted',
		accent: 'bg-accent-soft text-accent'
	};
</script>

<span
	class="inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-xs font-medium whitespace-nowrap {styles[
		tone
	]}"
	{title}
>
	<span class="size-1.5 rounded-full bg-current" aria-hidden="true"></span>
	{label}
</span>
