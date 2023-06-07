<script lang="ts">
	import { goto } from '$app/navigation';
	import { pageState } from '$lib/config';
	import { invoke } from '@tauri-apps/api/tauri';

	let unsavedData = false;
	let measuring = false;

	async function goBack() {
		if (unsavedData) {
			if (!(await confirm('You have unsaved data. Are you sure you want to leave?'))) {
				return;
			}
		}
		$pageState = 'connect';
		await goto('/connect');
	}

	async function start() {
		measuring = true;
		try {
			await invoke('start_event_loop');
		} catch (e) {
			console.error(e);
		}
	}

	async function stop() {
		measuring = false;
		try {
			await invoke('stop_event_loop');
		} catch (e) {
			console.error(e);
		}
	}
</script>

<svelte:head>
	<title>Sensor</title>
	<meta name="description" content="Track events from your heart rate sensor" />
</svelte:head>

<div class="text-column">
	<button on:click={goBack}>Back</button>

	<p>Head to <b>polar-tracker → File → Export</b> to save your data.</p>

	<button on:click={start} disabled={measuring}>Start Measurement</button>
	<button on:click={stop} disabled={!measuring}>Stop Measurement</button>
</div>
