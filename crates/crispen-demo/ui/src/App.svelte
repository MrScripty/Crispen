<script lang="ts">
  import ToolbarPanel from '$lib/docking/panels/ToolbarPanel.svelte';
  import DockviewContainer from '$lib/docking/DockviewContainer.svelte';
  import { bridge } from '$lib/bridge';
  import type { GradingParams } from '$lib/types';
  import { onMount } from 'svelte';

  // Backend-owned state (never modified locally, only received from Bevy)
  let params = $state<GradingParams | null>(null);
  let imageInfo = $state<{ path: string; width: number; height: number; bit_depth: string } | null>(null);

  // Transient UI state (local only)
  let error = $state<string | null>(null);

  onMount(() => {
    const unsubscribe = bridge.subscribe((msg) => {
      switch (msg.type) {
        case 'Initialize':
          params = msg.data.params;
          break;
        case 'ParamsUpdated':
          params = msg.data.params;
          break;
        case 'ImageLoaded':
          imageInfo = msg.data;
          break;
        case 'Error':
          error = msg.data.message;
          break;
      }
    });

    return unsubscribe;
  });
</script>

<div class="app">
  <ToolbarPanel {params} {imageInfo} {error} />
  <DockviewContainer {params} />
</div>

<style>
  .app {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: transparent;
    color: var(--color-text-primary);
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 13px;
  }
</style>
