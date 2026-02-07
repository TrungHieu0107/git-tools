import { writable } from 'svelte/store';

export const graphReloadRequested = writable<number>(0);

export function triggerGraphReload() {
    graphReloadRequested.update(n => n + 1);
}
