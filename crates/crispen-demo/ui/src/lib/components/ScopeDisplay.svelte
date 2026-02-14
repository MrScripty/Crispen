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

  let histCanvas: HTMLCanvasElement | undefined = $state();
  let waveCanvas: HTMLCanvasElement | undefined = $state();
  let vecCanvas: HTMLCanvasElement | undefined = $state();

  const CANVAS_WIDTH = 300;
  const CANVAS_HEIGHT = 150;
  const VEC_SIZE = 200;

  onMount(() => {
    drawHistogram();
    drawWaveform();
    drawVectorscope();
  });

  $effect(() => {
    if (data.histogram) drawHistogram();
  });

  $effect(() => {
    if (data.waveform) drawWaveform();
  });

  $effect(() => {
    if (data.vectorscope) drawVectorscope();
  });

  // ── Histogram ──────────────────────────────────────────────────

  function drawHistogram() {
    if (!histCanvas || !data.histogram) return;
    const ctx = histCanvas.getContext('2d');
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

  // ── Waveform ───────────────────────────────────────────────────

  function drawWaveform() {
    if (!waveCanvas || !data.waveform) return;
    const ctx = waveCanvas.getContext('2d');
    if (!ctx) return;

    const { width: srcW, height: srcH, data: channels } = data.waveform;
    const w = CANVAS_WIDTH;
    const h = CANVAS_HEIGHT;

    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = '#111';
    ctx.fillRect(0, 0, w, h);

    if (srcW === 0 || srcH === 0) return;

    // Find peak density across all channels for normalisation.
    let peak = 0;
    for (let ch = 0; ch < 3; ch++) {
      const arr = channels[ch];
      if (!arr) continue;
      for (let i = 0; i < arr.length; i++) {
        if (arr[i] > peak) peak = arr[i];
      }
    }
    if (peak === 0) return;

    // Render using ImageData for per-pixel control.
    const imgData = ctx.createImageData(w, h);
    const pixels = imgData.data;

    // Channel base colours: R, G, B
    const chColors = [
      [255, 80, 80],
      [80, 255, 80],
      [80, 120, 255],
    ];

    // Map canvas coords to waveform data coords and accumulate.
    for (let cy = 0; cy < h; cy++) {
      // Waveform y=0 is dark (bottom), y=srcH-1 is bright (top).
      // Canvas y=0 is top, y=h-1 is bottom. Flip.
      const srcY = Math.floor(((h - 1 - cy) / (h - 1)) * (srcH - 1));

      for (let cx = 0; cx < w; cx++) {
        const srcX = Math.floor((cx / w) * srcW);
        const srcIdx = srcY * srcW + srcX;

        let rAcc = 0, gAcc = 0, bAcc = 0;

        for (let ch = 0; ch < 3; ch++) {
          const density = channels[ch]?.[srcIdx] ?? 0;
          if (density === 0) continue;
          // Log scale for better visibility of low-density areas.
          const intensity = Math.min(1, Math.log1p(density) / Math.log1p(peak));
          rAcc += chColors[ch][0] * intensity;
          gAcc += chColors[ch][1] * intensity;
          bAcc += chColors[ch][2] * intensity;
        }

        if (rAcc > 0 || gAcc > 0 || bAcc > 0) {
          const idx = (cy * w + cx) * 4;
          pixels[idx] = Math.min(255, rAcc);
          pixels[idx + 1] = Math.min(255, gAcc);
          pixels[idx + 2] = Math.min(255, bAcc);
          pixels[idx + 3] = 255;
        }
      }
    }

    ctx.putImageData(imgData, 0, 0);

    // Draw 10% / 90% guide lines.
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
    ctx.setLineDash([4, 4]);
    const y10 = h - h * 0.1;
    const y90 = h - h * 0.9;
    ctx.beginPath();
    ctx.moveTo(0, y10);
    ctx.lineTo(w, y10);
    ctx.moveTo(0, y90);
    ctx.lineTo(w, y90);
    ctx.stroke();
    ctx.setLineDash([]);
  }

  // ── Vectorscope ────────────────────────────────────────────────

  function drawVectorscope() {
    if (!vecCanvas || !data.vectorscope) return;
    const ctx = vecCanvas.getContext('2d');
    if (!ctx) return;

    const { resolution, density } = data.vectorscope;
    const s = VEC_SIZE;

    ctx.clearRect(0, 0, s, s);
    ctx.fillStyle = '#111';
    ctx.fillRect(0, 0, s, s);

    if (resolution === 0 || density.length === 0) return;

    // Find peak density for normalisation.
    let peak = 0;
    for (let i = 0; i < density.length; i++) {
      if (density[i] > peak) peak = density[i];
    }

    if (peak > 0) {
      // Render density as green luminance map using ImageData.
      const imgData = ctx.createImageData(s, s);
      const pixels = imgData.data;

      for (let cy = 0; cy < s; cy++) {
        const srcY = Math.floor((cy / s) * resolution);
        for (let cx = 0; cx < s; cx++) {
          const srcX = Math.floor((cx / s) * resolution);
          const d = density[srcY * resolution + srcX] ?? 0;
          if (d === 0) continue;

          const intensity = Math.min(1, Math.log1p(d) / Math.log1p(peak));
          const idx = (cy * s + cx) * 4;
          pixels[idx] = Math.round(80 * intensity);
          pixels[idx + 1] = Math.round(255 * intensity);
          pixels[idx + 2] = Math.round(80 * intensity);
          pixels[idx + 3] = 255;
        }
      }

      ctx.putImageData(imgData, 0, 0);
    }

    // Graticule: circle and crosshair.
    const cx = s / 2;
    const cy = s / 2;
    const r = s / 2 - 4;

    ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
    ctx.lineWidth = 1;

    // Outer circle
    ctx.beginPath();
    ctx.arc(cx, cy, r, 0, Math.PI * 2);
    ctx.stroke();

    // Inner circle (50%)
    ctx.beginPath();
    ctx.arc(cx, cy, r * 0.5, 0, Math.PI * 2);
    ctx.stroke();

    // Crosshair
    ctx.beginPath();
    ctx.moveTo(cx, cy - r);
    ctx.lineTo(cx, cy + r);
    ctx.moveTo(cx - r, cy);
    ctx.lineTo(cx + r, cy);
    ctx.stroke();

    // Skin tone line (~123° from positive Cb axis, or ~33° from top).
    // In Cb/Cr space: angle ≈ 123° from +Cb (x-axis), which maps to
    // a line from center toward upper-left in standard vectorscope orientation.
    ctx.strokeStyle = 'rgba(255, 180, 100, 0.3)';
    ctx.lineWidth = 1;
    const skinAngle = (123 * Math.PI) / 180;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx + r * Math.cos(skinAngle), cy - r * Math.sin(skinAngle));
    ctx.stroke();

    // Colour target markers (R, G, B, C, M, Y at 75% bars).
    const targets = [
      { label: 'R', angle: 103, color: 'rgba(255,80,80,0.6)' },
      { label: 'YL', angle: 167, color: 'rgba(255,255,80,0.5)' },
      { label: 'G', angle: 241, color: 'rgba(80,255,80,0.5)' },
      { label: 'CY', angle: 283, color: 'rgba(80,255,255,0.5)' },
      { label: 'B', angle: 347, color: 'rgba(80,120,255,0.6)' },
      { label: 'MG', angle: 61, color: 'rgba(255,80,255,0.5)' },
    ];

    for (const t of targets) {
      const a = (t.angle * Math.PI) / 180;
      const tx = cx + r * 0.75 * Math.cos(a);
      const ty = cy - r * 0.75 * Math.sin(a);
      ctx.fillStyle = t.color;
      ctx.beginPath();
      ctx.arc(tx, ty, 3, 0, Math.PI * 2);
      ctx.fill();
    }
  }
</script>

<div class="scope-display">
  <h3>Scopes</h3>

  <div class="scope-section">
    <span class="scope-label">Histogram</span>
    <canvas bind:this={histCanvas} width={CANVAS_WIDTH} height={CANVAS_HEIGHT}></canvas>
    {#if !data.histogram}
      <p class="no-data">No histogram data</p>
    {/if}
  </div>

  <div class="scope-section">
    <span class="scope-label">Waveform</span>
    <canvas bind:this={waveCanvas} width={CANVAS_WIDTH} height={CANVAS_HEIGHT}></canvas>
    {#if !data.waveform}
      <p class="no-data">No waveform data</p>
    {/if}
  </div>

  <div class="scope-section">
    <span class="scope-label">Vectorscope</span>
    <canvas bind:this={vecCanvas} width={VEC_SIZE} height={VEC_SIZE}></canvas>
    {#if !data.vectorscope}
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

  .no-data {
    font-size: 11px;
    color: #444;
    margin: 4px 0;
  }
</style>
