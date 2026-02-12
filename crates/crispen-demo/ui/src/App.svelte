<script lang="ts">
  import ColorWheels from '$lib/components/ColorWheels.svelte';
  import PrimaryBars from '$lib/components/PrimaryBars.svelte';
  import Sliders from '$lib/components/Sliders.svelte';
  import CurveEditor from '$lib/components/CurveEditor.svelte';
  import ScopeDisplay from '$lib/components/ScopeDisplay.svelte';
  import ColorSpaceSelector from '$lib/components/ColorSpaceSelector.svelte';
  import { bridge } from '$lib/bridge';
  import type {
    GradingParams,
    HistogramData,
    WaveformData,
    VectorscopeData,
    CieData,
  } from '$lib/types';
  import { onMount } from 'svelte';

  // Backend-owned state (never modified locally, only received from Bevy)
  let params = $state<GradingParams | null>(null);
  let imageInfo = $state<{ width: number; height: number; bit_depth: string } | null>(null);
  let scopeData = $state<{
    histogram: HistogramData | null;
    waveform: WaveformData | null;
    vectorscope: VectorscopeData | null;
    cie: CieData | null;
  }>({ histogram: null, waveform: null, vectorscope: null, cie: null });

  // Transient UI state (local only)
  let error = $state<string | null>(null);
  let imagePath = $state('');

  function loadImageFromPath() {
    const path = imagePath.trim();
    if (!path) {
      return;
    }
    bridge.loadImage(path);
  }

  onMount(() => {
    const unsubscribe = bridge.subscribe((msg) => {
      switch (msg.type) {
        case 'Initialize':
          params = msg.data.params;
          break;
        case 'ParamsUpdated':
          params = msg.data.params;
          break;
        case 'ScopeData':
          scopeData = msg.data;
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
  <header class="toolbar">
    <h1>Crispen</h1>
    <div class="toolbar-actions">
      <button onclick={() => bridge.autoBalance()}>Auto Balance</button>
      <button onclick={() => bridge.resetGrade()}>Reset</button>
      <input
        class="path-input"
        type="text"
        placeholder="Image path..."
        bind:value={imagePath}
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            loadImageFromPath();
          }
        }}
      />
      <button onclick={loadImageFromPath}>Load Image</button>
    </div>
    {#if imageInfo}
      <span class="image-info">
        {imageInfo.width}&times;{imageInfo.height} ({imageInfo.bit_depth})
      </span>
    {/if}
    {#if error}
      <span class="error-badge">{error}</span>
    {/if}
  </header>

  <main class="workspace">
    <section class="scopes-panel">
      <ScopeDisplay data={scopeData} />
    </section>

    <section class="controls-panel">
      {#if params}
        <ColorSpaceSelector {params} />
        <ColorWheels {params} />
        <PrimaryBars {params} />
        <Sliders {params} />
        <CurveEditor {params} />
      {:else}
        <p class="loading">Connecting to Bevy backend...</p>
      {/if}
    </section>
  </main>
</div>

<style>
  .app {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #1a1a1a;
    color: #e0e0e0;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 13px;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    background: #252525;
    border-bottom: 1px solid #333;
    flex-shrink: 0;
  }

  .toolbar h1 {
    font-size: 16px;
    font-weight: 500;
    margin: 0;
    color: #fff;
  }

  .toolbar-actions {
    display: flex;
    gap: 6px;
  }

  .toolbar-actions button {
    padding: 4px 12px;
    background: #3a3a3a;
    border: 1px solid #555;
    border-radius: 4px;
    color: #e0e0e0;
    cursor: pointer;
    font-size: 12px;
  }

  .toolbar-actions .path-input {
    width: 360px;
    max-width: 40vw;
    padding: 4px 8px;
    background: #1e1e1e;
    border: 1px solid #555;
    border-radius: 4px;
    color: #e0e0e0;
    font-size: 12px;
  }

  .toolbar-actions button:hover {
    background: #4a4a4a;
  }

  .image-info {
    margin-left: auto;
    color: #888;
    font-size: 12px;
  }

  .error-badge {
    color: #ff6b6b;
    font-size: 12px;
  }

  .workspace {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .scopes-panel {
    width: 320px;
    flex-shrink: 0;
    border-right: 1px solid #333;
    overflow-y: auto;
    padding: 8px;
  }

  .controls-panel {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .loading {
    color: #666;
    text-align: center;
    padding: 40px;
  }
</style>
