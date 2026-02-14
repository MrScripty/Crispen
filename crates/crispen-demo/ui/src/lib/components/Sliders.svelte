<script lang="ts">
  import type { GradingParams } from '$lib/types';
  import { bridge } from '$lib/bridge';

  let { params }: { params: GradingParams } = $props();

  const sliders = [
    { key: 'temperature' as const, label: 'Temperature', min: -100, max: 100, step: 1 },
    { key: 'tint' as const, label: 'Tint', min: -100, max: 100, step: 1 },
    { key: 'contrast' as const, label: 'Contrast', min: 0, max: 4, step: 0.01 },
    { key: 'pivot' as const, label: 'Pivot', min: 0, max: 1, step: 0.001 },
    { key: 'midtone_detail' as const, label: 'Midtone Detail', min: -100, max: 100, step: 1 },
    { key: 'shadows' as const, label: 'Shadows', min: -100, max: 100, step: 1 },
    { key: 'highlights' as const, label: 'Highlights', min: -100, max: 100, step: 1 },
    { key: 'saturation' as const, label: 'Saturation', min: 0, max: 4, step: 0.01 },
    { key: 'hue' as const, label: 'Hue', min: -180, max: 180, step: 1 },
    { key: 'luma_mix' as const, label: 'Luma Mix', min: 0, max: 1, step: 0.01 },
  ];

  type SliderKey = (typeof sliders)[number]['key'];

  function updateSlider(key: SliderKey, value: number) {
    const updated = $state.snapshot(params) as GradingParams;
    updated[key] = value;
    bridge.setParams(updated);
  }
</script>

<div class="sliders">
  <h3>Adjustments</h3>
  {#each sliders as slider}
    <label class="slider-row">
      <span class="slider-label">{slider.label}</span>
      <input
        type="range"
        min={slider.min}
        max={slider.max}
        step={slider.step}
        value={params[slider.key]}
        oninput={(e) =>
          updateSlider(slider.key, parseFloat((e.target as HTMLInputElement).value))}
      />
      <span class="slider-value">{params[slider.key].toFixed(2)}</span>
    </label>
  {/each}
</div>

<style>
  .sliders h3 {
    margin: 16px 0 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--color-text-heading);
  }

  .slider-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
    cursor: pointer;
  }

  .slider-label {
    width: 110px;
    font-size: 11px;
    color: var(--color-text-secondary);
    flex-shrink: 0;
  }

  .slider-row input[type='range'] {
    flex: 1;
    height: 14px;
    cursor: pointer;
  }

  .slider-value {
    width: 50px;
    text-align: right;
    font-size: 11px;
    color: var(--color-text-value);
    font-variant-numeric: tabular-nums;
  }
</style>
