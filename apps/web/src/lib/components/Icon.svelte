<script lang="ts">
	import { ICONS, type IconName } from './icons';

	let { name, size = 18, label }: { name: IconName; size?: number; label?: string } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="1.7"
	stroke-linecap="round"
	stroke-linejoin="round"
	role={label ? 'img' : undefined}
	aria-label={label}
	aria-hidden={label ? undefined : 'true'}
>
	<path d={ICONS[name]} />
</svg>
