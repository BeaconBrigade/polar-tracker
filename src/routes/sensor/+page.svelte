<script lang="ts">
	import { goto } from '$app/navigation';
	import { clearConfig, config, pageState } from '$lib/config';

	let unsavedData = false;

	async function goBack() {
		if (unsavedData) {
			if (!await confirm('You have unsaved data. Are you sure you want to leave?')) {
				return;
			}
			clearConfig();
		}
		$pageState = 'connect';
		await goto('/connect');
	}
</script>

<svelte:head>
	<title>Sensor</title>
	<meta name="description" content="Track events from your heart rate sensor" />
</svelte:head>

<div class="text-column">
	<button on:click={goBack}>Back</button>
	<p>What is up</p>
	<pre>{JSON.stringify($config, null, 2)}</pre>
</div>
