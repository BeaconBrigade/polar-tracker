import { writable } from 'svelte/store';

export type Config = {
	participantId: string;
	sessionNumber: number;
	trialId: number;
	description: string;
	range: '2' | '4' | '8';
	rate: '25' | '50' | '100' | '200';
};

export const config = writable<Config | undefined>(undefined);

export function setConfig(val: Config) {
	config.set(val);
}

export function clearConfig() {
	config.set(undefined);
}

export const pageState = writable<'form' | 'connect' | 'sensor'>('form');

export const connected = writable<boolean>(false);

export const deviceID = writable<string>('');
