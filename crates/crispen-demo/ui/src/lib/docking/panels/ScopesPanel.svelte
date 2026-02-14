<!--
  Dockable panel wrapping the ScopeDisplay component.
-->
<script lang="ts">
  import ScopeDisplay from '$lib/components/ScopeDisplay.svelte';
  import type { HistogramData, WaveformData, VectorscopeData, CieData } from '$lib/types';

  let {
    data,
  }: {
    data: {
      histogram: HistogramData | null;
      waveform: WaveformData | null;
      vectorscope: VectorscopeData | null;
      cie: CieData | null;
    };
  } = $props();
</script>

<div class="svelte-panel">
  <ScopeDisplay {data} />
</div>

<style>
  .svelte-panel {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    pointer-events: auto;
    padding: 8px;
  }
</style>
