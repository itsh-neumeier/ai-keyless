<script lang="ts">
	let { message }: { message: string | null } = $props();
</script>

{#if message}
	<p role="alert" class="rounded-md bg-danger-soft px-3 py-2 text-[13px] text-danger">{message}</p>
{/if}
