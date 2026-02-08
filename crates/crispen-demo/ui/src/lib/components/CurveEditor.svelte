<script lang="ts">
  import type { GradingParams } from '$lib/types';

  let { params }: { params: GradingParams } = $props();

  const curves = [
    { key: 'hue_vs_hue' as const, label: 'Hue vs Hue' },
    { key: 'hue_vs_sat' as const, label: 'Hue vs Sat' },
    { key: 'lum_vs_sat' as const, label: 'Lum vs Sat' },
    { key: 'sat_vs_sat' as const, label: 'Sat vs Sat' },
  ];

  let activeCurve = $state(0);
</script>

<div class="curve-editor">
  <h3>Curves</h3>
  <div class="curve-tabs">
    {#each curves as curve, i}
      <button
        class="curve-tab"
        class:active={activeCurve === i}
        onclick={() => (activeCurve = i)}
      >
        {curve.label}
      </button>
    {/each}
  </div>
  <div class="curve-canvas">
    <div class="placeholder">
      <p>{curves[activeCurve].label}</p>
      <p class="hint">
        {params[curves[activeCurve].key].length} control points
      </p>
    </div>
  </div>
</div>

<style>
  .curve-editor h3 {
    margin: 16px 0 8px;
    font-size: 13px;
    font-weight: 500;
    color: #aaa;
  }

  .curve-tabs {
    display: flex;
    gap: 2px;
    margin-bottom: 8px;
  }

  .curve-tab {
    padding: 4px 8px;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 3px;
    color: #888;
    cursor: pointer;
    font-size: 10px;
  }

  .curve-tab.active {
    background: #3a3a3a;
    color: #e0e0e0;
    border-color: #666;
  }

  .curve-canvas {
    width: 100%;
    aspect-ratio: 1;
    background: #1a1a1a;
    border: 1px solid #333;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .placeholder {
    text-align: center;
    color: #555;
  }

  .placeholder p {
    margin: 4px 0;
  }

  .hint {
    font-size: 11px;
    color: #444;
  }
</style>
