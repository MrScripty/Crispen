<script lang="ts">
  import type { ColorManagementConfig } from '$lib/types';

  let { config }: { config: ColorManagementConfig } = $props();

  const colorSpaces = [
    'Aces2065_1',
    'AcesCg',
    'AcesCc',
    'AcesCct',
    'Srgb',
    'LinearSrgb',
    'Rec2020',
    'DciP3',
    'ArriLogC3',
    'ArriLogC4',
    'SLog3',
    'RedLog3G10',
    'VLog',
  ];

  const labels: Record<string, string> = {
    Aces2065_1: 'ACES 2065-1',
    AcesCg: 'ACEScg',
    AcesCc: 'ACEScc',
    AcesCct: 'ACEScct',
    Srgb: 'sRGB',
    LinearSrgb: 'Linear sRGB',
    Rec2020: 'Rec. 2020',
    DciP3: 'DCI-P3',
    ArriLogC3: 'ARRI LogC3',
    ArriLogC4: 'ARRI LogC4',
    SLog3: 'S-Log3',
    RedLog3G10: 'RED Log3G10',
    VLog: 'V-Log',
  };

  function updateSpace(field: keyof ColorManagementConfig, value: string) {
    // We send a full params update with the new color management config.
    // Since we only have the config prop, we construct a minimal update.
    // The backend will merge this with the current params.
    const newConfig = { ...config, [field]: value };
    // NOTE: In a full implementation, we'd need access to full params here.
    // For now, we log the intent. The parent App.svelte should pass the
    // full params and handle the update.
    console.warn('ColorSpaceSelector: would update', field, 'to', value, newConfig);
  }
</script>

<div class="color-space-selector">
  <h3>Color Management</h3>
  <div class="selector-row">
    <label>
      <span>Input</span>
      <select
        value={config.input_space}
        onchange={(e) => updateSpace('input_space', (e.target as HTMLSelectElement).value)}
      >
        {#each colorSpaces as cs}
          <option value={cs}>{labels[cs] ?? cs}</option>
        {/each}
      </select>
    </label>
  </div>
  <div class="selector-row">
    <label>
      <span>Working</span>
      <select
        value={config.working_space}
        onchange={(e) => updateSpace('working_space', (e.target as HTMLSelectElement).value)}
      >
        {#each colorSpaces as cs}
          <option value={cs}>{labels[cs] ?? cs}</option>
        {/each}
      </select>
    </label>
  </div>
  <div class="selector-row">
    <label>
      <span>Output</span>
      <select
        value={config.output_space}
        onchange={(e) => updateSpace('output_space', (e.target as HTMLSelectElement).value)}
      >
        {#each colorSpaces as cs}
          <option value={cs}>{labels[cs] ?? cs}</option>
        {/each}
      </select>
    </label>
  </div>
</div>

<style>
  .color-space-selector h3 {
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 500;
    color: #aaa;
  }

  .selector-row {
    margin-bottom: 6px;
  }

  .selector-row label {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .selector-row span {
    width: 60px;
    font-size: 11px;
    color: #888;
  }

  .selector-row select {
    flex: 1;
    padding: 3px 6px;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 3px;
    color: #e0e0e0;
    font-size: 11px;
  }
</style>
