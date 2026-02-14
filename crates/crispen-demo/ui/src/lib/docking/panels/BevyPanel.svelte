<!--
  Transparent cutout panel for Bevy-rendered GPU widgets.
  Reports its dimensions to the bridge so Bevy can position the native widget underneath.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { bridge } from '$lib/bridge';
  import type { LayoutRegion } from '$lib/types';

  let { panelId }: { panelId: string } = $props();

  let container: HTMLDivElement | undefined = $state();

  function reportBounds() {
    if (!container) return;
    const rect = container.getBoundingClientRect();
    const region: LayoutRegion = {
      id: panelId,
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
      visible: true,
    };
    bridge.updateLayout([region]);
  }

  onMount(() => {
    if (!container) return;

    const observer = new ResizeObserver(() => reportBounds());
    observer.observe(container);
    reportBounds();

    // Re-report bounds when the CEF IPC bridge becomes available.
    // Initial reportBounds() may fire before inject_ipc_bridge() runs,
    // so the LayoutUpdate message would be silently dropped.
    const onIpcReady = () => reportBounds();
    window.addEventListener('crispen-ipc-ready', onIpcReady);

    return () => {
      observer.disconnect();
      window.removeEventListener('crispen-ipc-ready', onIpcReady);
    };
  });
</script>

<div class="bevy-panel" bind:this={container}>
  <span class="bevy-panel-label">{panelId}</span>
</div>

<style>
  .bevy-panel {
    width: 100%;
    height: 100%;
    pointer-events: none;
    background: transparent;
    position: relative;
  }

  .bevy-panel-label {
    position: absolute;
    top: 4px;
    left: 8px;
    font-size: 10px;
    color: var(--color-panel-label);
    pointer-events: none;
    user-select: none;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
</style>
