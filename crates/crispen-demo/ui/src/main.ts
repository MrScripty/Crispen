import App from './App.svelte';
import { mount } from 'svelte';
import { setupAutoMarkDirty } from '$lib/bridge';

const app = mount(App, {
  target: document.getElementById('app')!,
});

setupAutoMarkDirty();

export default app;
