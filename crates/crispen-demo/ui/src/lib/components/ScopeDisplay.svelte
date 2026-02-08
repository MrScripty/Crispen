<script lang="ts">
  import type { HistogramData, WaveformData, VectorscopeData, CieData } from '$lib/types';
  import { onMount } from 'svelte';

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

  let canvas: HTMLCanvasElement | undefined = $state();

  const CANVAS_WIDTH = 300;
  const CANVAS_HEIGHT = 150;

  onMount(() => {
    drawHistogram();
  });

  $effect(() => {
    if (data.histogram) {
      drawHistogram();
    }
  });

  function drawHistogram() {
    if (!canvas || !data.histogram) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const { bins, peak } = data.histogram;
    const w = CANVAS_WIDTH;
    const h = CANVAS_HEIGHT;

    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = '#111';
    ctx.fillRect(0, 0, w, h);

    if (peak === 0) return;

    const colors = ['rgba(255,80,80,0.6)', 'rgba(80,255,80,0.6)', 'rgba(80,120,255,0.6)'];
    const binCount = bins[0]?.length ?? 256;
    const barWidth = w / binCount;

    for (let ch = 0; ch < 3; ch++) {
      ctx.fillStyle = colors[ch];
      for (let i = 0; i < binCount; i++) {
        const value = bins[ch]?.[i] ?? 0;
        const barHeight = (value / peak) * h;
        ctx.fillRect(i * barWidth, h - barHeight, barWidth, barHeight);
      }
    }
  }
</script>

<div class="scope-display">
  <h3>Scopes</h3>

  <div class="scope-section">
    <span class="scope-label">Histogram</span>
    <canvas bind:this={canvas} width={CANVAS_WIDTH} height={CANVAS_HEIGHT}></canvas>
    {#if !data.histogram}
      <p class="no-data">No histogram data</p>
    {/if}
  </div>

  <div class="scope-section">
    <span class="scope-label">Waveform</span>
    {#if data.waveform}
      <p class="scope-info">{data.waveform.width}x{data.waveform.height}</p>
    {:else}
      <p class="no-data">No waveform data</p>
    {/if}
  </div>

  <div class="scope-section">
    <span class="scope-label">Vectorscope</span>
    {#if data.vectorscope}
      <p class="scope-info">Resolution: {data.vectorscope.resolution}</p>
    {:else}
      <p class="no-data">No vectorscope data</p>
    {/if}
  </div>
</div>

<style>
  .scope-display h3 {
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 500;
    color: #aaa;
  }

  .scope-section {
    margin-bottom: 12px;
  }

  .scope-label {
    display: block;
    font-size: 10px;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 4px;
  }

  canvas {
    width: 100%;
    height: auto;
    border-radius: 3px;
    display: block;
  }

  .no-data,
  .scope-info {
    font-size: 11px;
    color: #444;
    margin: 4px 0;
  }
</style>
