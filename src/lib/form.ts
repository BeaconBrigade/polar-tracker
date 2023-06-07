import { writable } from 'svelte/store';
import type { Config } from './config';

export const formConfig = writable<Config>({
	participantId: '',
	sessionNumber: 0,
	trialId: 0,
	description: '',
	range: '8',
	rate: '200'
});

export function clearForm() {
	formConfig.set({
		participantId: '',
		sessionNumber: 0,
		trialId: 0,
		description: '',
		range: '8',
		rate: '200'
	});
}
