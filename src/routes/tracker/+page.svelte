<script lang="ts">
	import { goto } from '$app/navigation';
	import { pageState, setConfig, config } from '$lib/config';
	import { clearForm, formConfig } from '$lib/form';
	import { invoke } from '@tauri-apps/api/tauri';

	async function handleSubmit(e: Event) {
		e.preventDefault();

		setConfig($formConfig);
        let temp = $formConfig;
        temp.rate = +$formConfig.rate;
        temp.range = +$formConfig.range;
        console.log(JSON.stringify(temp, null, 2));
		await invoke('set_config', { config: temp });
		clearForm();
		$pageState = 'connect';
		await goto('/connect');
	}
</script>

<svelte:head>
	<title>Tracker</title>
	<meta name="description" content="Set up a tracker for polar heart rate sensor" />
</svelte:head>

<div class="text-column">
	<h1>Tracker Information</h1>

	<form on:submit={handleSubmit}>
		<fieldset>
			<label for="participant">Participant ID</label>
			<input id="participant" type="text" required bind:value={$formConfig.participantId} />
		</fieldset>

		<fieldset>
			<label for="session">Session Number</label>
			<input id="session" type="number" required bind:value={$formConfig.sessionNumber} />
		</fieldset>

		<fieldset>
			<label for="trial">Trial ID</label>
			<input id="trial" type="number" required bind:value={$formConfig.trialId} />
		</fieldset>

		<fieldset>
			<label for="description">Description</label>
			<input id="description" type="text" required bind:value={$formConfig.description} />
		</fieldset>

		<fieldset>
			<label for="range">Select range to measure acceleration</label>
			<select id="range" bind:value={$formConfig.range}>
				<option value="8">8 Gs</option>
				<option value="4">4 Gs</option>
				<option value="2">2 Gs</option>
			</select>
		</fieldset>

		<fieldset>
			<label for="rate">Select sample rate for acceleration</label>
			<select id="rate" bind:value={$formConfig.rate}>
				<option value="200">200 hz</option>
				<option value="100">100 hz</option>
				<option value="50">50 hz</option>
				<option value="25">25 hz</option>
			</select>
		</fieldset>

		<fieldset class="submit">
			<input id="submit" type="submit" value="Next" />
		</fieldset>
	</form>
</div>

<style>
	fieldset {
		display: flex;
		margin: 10px;
		padding: 20px;
		align-items: center;
	}

	input,
	select {
		margin-left: 20px;
	}

	.submit {
		margin: 10px;
		align-items: center;
		justify-content: center;
	}
</style>
