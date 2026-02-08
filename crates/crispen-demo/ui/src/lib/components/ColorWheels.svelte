<script lang="ts">
  import type { GradingParams } from '$lib/types';
  import { bridge } from '$lib/bridge';

  let { params }: { params: GradingParams } = $props();

  const wheels = ['lift', 'gamma', 'gain', 'offset'] as const;
  const channels = ['R', 'G', 'B', 'M'] as const;

  function updateWheel(wheel: (typeof wheels)[number], channel: number, value: number) {
    const updated = structuredClone(params);
    updated[wheel][channel] = value;
    bridge.setParams(updated);
  }
</script>

<div class="color-wheels">
  <h3>Color Wheels</h3>
  <div class="wheels-grid">
    {#each wheels as wheel}
      <div class="wheel-group">
        <span class="wheel-label">{wheel}</span>
        <div class="wheel-channels">
          {#each channels as ch, i}
            <label class="channel">
              <span class="channel-label">{ch}</span>
              <input
                type="number"
                step="0.01"
                value={params[wheel][i]}
                onchange={(e) =>
                  updateWheel(wheel, i, parseFloat((e.target as HTMLInputElement).value))}
              />
            </label>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .color-wheels h3 {
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 500;
    color: #aaa;
  }

  .wheels-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .wheel-group {
    background: #252525;
    border-radius: 6px;
    padding: 8px;
  }

  .wheel-label {
    display: block;
    font-size: 11px;
    color: #888;
    text-transform: capitalize;
    margin-bottom: 6px;
  }

  .wheel-channels {
    display: flex;
    gap: 4px;
  }

  .channel {
    display: flex;
    flex-direction: column;
    align-items: center;
    flex: 1;
  }

  .channel-label {
    font-size: 10px;
    color: #666;
    margin-bottom: 2px;
  }

  .channel input {
    width: 100%;
    padding: 2px 4px;
    background: #1a1a1a;
    border: 1px solid #444;
    border-radius: 3px;
    color: #e0e0e0;
    font-size: 11px;
    text-align: center;
  }
</style>
