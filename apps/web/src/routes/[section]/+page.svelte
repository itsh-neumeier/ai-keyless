<script lang="ts">
	import Placeholder from '#lib/components/Placeholder.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<Placeholder item={data.item} />
