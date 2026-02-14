<!--
  Top toolbar: actions (Auto Balance, Reset, Load Image) and image info.
  Not dockable — pinned at the top of the layout.
-->
<script lang="ts">
  import ColorSpaceSelector from '$lib/components/ColorSpaceSelector.svelte';
  import { bridge } from '$lib/bridge';
  import type { GradingParams } from '$lib/types';

  let {
    params,
    imageInfo,
    error,
  }: {
    params: GradingParams | null;
    imageInfo: { path: string; width: number; height: number; bit_depth: string } | null;
    error: string | null;
  } = $props();

  let imagePath = $state('');

  // Sync imagePath when the backend reports a loaded image (e.g. from Ctrl+O).
  $effect(() => {
    if (imageInfo?.path) {
      imagePath = imageInfo.path;
    }
  });

  function loadImageFromPath() {
    const path = imagePath.trim();
    if (!path) return;
    bridge.loadImage(path);
  }
</script>

<header class="toolbar">
  <h1>Crispen</h1>
  <div class="toolbar-actions">
    {#if params}
      <ColorSpaceSelector {params} />
    {/if}
    <button onclick={() => bridge.autoBalance()}>Auto Balance</button>
    <button onclick={() => bridge.resetGrade()}>Reset</button>
    <input
      class="path-input"
      type="text"
      placeholder="Image path..."
      bind:value={imagePath}
      onkeydown={(e) => {
        if (e.key === 'Enter') loadImageFromPath();
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

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    background: #252525;
    border-bottom: 1px solid #333;
    flex-shrink: 0;
    pointer-events: auto;
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
    align-items: center;
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

  .toolbar-actions button:hover {
    background: #4a4a4a;
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

  .image-info {
    margin-left: auto;
    color: #888;
    font-size: 12px;
  }

  .error-badge {
    color: #ff6b6b;
    font-size: 12px;
  }
</style>
