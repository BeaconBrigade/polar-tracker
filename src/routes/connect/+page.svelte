<script lang="ts">
	import { goto } from '$app/navigation';
	import { clearConfig, config, pageState } from '$lib/config';
	import { invoke } from '@tauri-apps/api/tauri';

	let deviceId = '';
	let connected = false;
    let isConnectionInProgress = false;
	$: unsavedData = deviceId !== '';

	async function goBack() {
		if (unsavedData) {
			if (!(await confirm('You have unsaved data. Are you sure you want to leave?'))) {
				return;
			}
			clearConfig();
		}
		if (connected) {
			await disconnect();
		}
		$pageState = 'form';
		await goto('/tracker');
	}

	async function next() {
		if (connected) {
			$pageState = 'sensor';
			await goto('/sensor');
		}
	}

	async function connect() {
        // don't block on future until updating ui state
        isConnectionInProgress = true;
		let res =  invoke('connect', { deviceId: deviceId });
		alert('Connecting');

        await res;
        isConnectionInProgress = false;
		connected = true;
	}

	async function disconnect() {
		let ans = await confirm('Are you sure you want to disconnect?');
		if (!ans) {
			return;
		}
		console.log('hey!!');
		connected = false;
	}
</script>

<svelte:head>
	<title>Connect</title>
	<meta name="description" content="Connect to your polar heart rate monitor" />
</svelte:head>

<div class="text-column">
	<h1>Connect to Polar Device</h1>
	<button on:click={goBack}>Back</button>

	<label for="device-id">Device ID</label>
	<input
		id="device-id"
		type="text"
		autocorrect="off"
		autocomplete="off"
		spellcheck="false"
		bind:value={deviceId}
	/>

	<pre>{JSON.stringify($config, null, 2)}</pre>

	{#if connected}
		<button on:click={disconnect}>Disconnect</button>
	{:else}
		<button on:click={connect} disabled={isConnectionInProgress}>Connect</button>
	{/if}
	<button on:click={next} disabled={!connected}>Next</button>
</div>
