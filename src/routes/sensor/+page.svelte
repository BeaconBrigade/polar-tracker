<script lang="ts">
	import { goto } from '$app/navigation';
	import { clearConfig, config, pageState } from '$lib/config';
	import { invoke } from '@tauri-apps/api/tauri';

	let unsavedData = false;
    let measuring = false;

	async function goBack() {
		if (unsavedData) {
			if (!(await confirm('You have unsaved data. Are you sure you want to leave?'))) {
				return;
			}
			clearConfig();
		}
		$pageState = 'connect';
		await goto('/connect');
	}

    async function start() {
        measuring = true;
        await invoke('start_event_loop');
        console.log("here");
    }
</script>

<svelte:head>
	<title>Sensor</title>
	<meta name="description" content="Track events from your heart rate sensor" />
</svelte:head>

<div class="text-column">
	<button on:click={goBack}>Back</button>
	<pre>{JSON.stringify($config, null, 2)}</pre>

    <button on:click={start} disabled={measuring}>Start Measurement</button>
</div>
