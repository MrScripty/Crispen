<script lang="ts">
  import type { GradingParams } from '$lib/types';
  import { bridge } from '$lib/bridge';

  let { params }: { params: GradingParams } = $props();

  const bars = [
    { key: 'lift' as const, label: 'Lift', min: -1, max: 1, step: 0.01 },
    { key: 'gamma' as const, label: 'Gamma', min: 0, max: 4, step: 0.01 },
    { key: 'gain' as const, label: 'Gain', min: 0, max: 4, step: 0.01 },
    { key: 'offset' as const, label: 'Offset', min: -1, max: 1, step: 0.01 },
  ];
  const channelVars = [
    'var(--color-channel-r)',
    'var(--color-channel-g)',
    'var(--color-channel-b)',
    'var(--color-channel-master)',
  ];

  function updateBar(key: 'lift' | 'gamma' | 'gain' | 'offset', channel: number, value: number) {
    const updated = $state.snapshot(params) as GradingParams;
    updated[key][channel] = value;
    bridge.setParams(updated);
  }
</script>

<div class="primary-bars">
  <h3>Primary Bars</h3>
  {#each bars as bar}
    <div class="bar-group">
      <span class="bar-label">{bar.label}</span>
      <div class="bar-sliders">
        {#each [0, 1, 2, 3] as ch}
          <input
            type="range"
            min={bar.min}
            max={bar.max}
            step={bar.step}
            value={params[bar.key][ch]}
            style="accent-color: {channelVars[ch]}"
            oninput={(e) => updateBar(bar.key, ch, parseFloat((e.target as HTMLInputElement).value))}
          />
        {/each}
      </div>
    </div>
  {/each}
</div>

<style>
  .primary-bars h3 {
    margin: 16px 0 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--color-text-heading);
  }

  .bar-group {
    margin-bottom: 8px;
  }

  .bar-label {
    display: block;
    font-size: 11px;
    color: var(--color-text-secondary);
    margin-bottom: 4px;
  }

  .bar-sliders {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .bar-sliders input[type='range'] {
    width: 100%;
    height: 14px;
    cursor: pointer;
  }
</style>
