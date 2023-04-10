<script lang="ts">
	import { goto } from '$app/navigation';
	import { pageState, setConfig } from '$lib/config';

	let participantId: string;
	let sessionNumber: number;
	let trialId: number;
	let description: string;
	let measureHr = false;
	let measureAcc = false;
	let measureEcg = false;
	let range: '2' | '4' | '8';
	let rate: '25' | '50' | '100' | '200';

	async function handleSubmit(e: Event) {
		e.preventDefault();
		if (!(measureHr || measureAcc || measureEcg)) {
			console.warn('choose either heartrate, acceleration or electrocardiagram to measure');
			return;
		}

		let res = {
			participantId,
			sessionNumber,
			trialId,
			description,
			measureHr,
			measureAcc,
			measureEcg,
			range,
			rate
		};

		setConfig(res);

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
			<input id="participant" type="text" required bind:value={participantId} />
		</fieldset>

		<fieldset>
			<label for="session">Session Number</label>
			<input id="session" type="number" required bind:value={sessionNumber} />
		</fieldset>

		<fieldset>
			<label for="trial">Trial ID</label>
			<input id="trial" type="number" required bind:value={trialId} />
		</fieldset>

		<fieldset>
			<label for="description">Description</label>
			<input id="description" type="text" required bind:value={description} />
		</fieldset>

		<fieldset>
			<label for="heart-rate">Measure Heart Rate</label>
			<input id="heart-rate" type="checkbox" bind:checked={measureHr} />
		</fieldset>

		<fieldset>
			<label for="acceleration">Measure Acceleration</label>
			<input id="acceleration" type="checkbox" bind:checked={measureAcc} />
		</fieldset>

		<fieldset>
			<label for="ecg">Measure ECG</label>
			<input id="ecg" type="checkbox" bind:checked={measureEcg} />
		</fieldset>

		<fieldset>
			<label for="range">Select range to measure acceleration</label>
			<select id="range" bind:value={range}>
				<option value="8">8 Gs</option>
				<option value="4">4 Gs</option>
				<option value="2">2 Gs</option>
			</select>
		</fieldset>

		<fieldset>
			<label for="rate">Select sample rate for acceleration</label>
			<select id="rate" bind:value={rate}>
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
