<script lang="ts">
  import type { GradingParams } from '$lib/types';
  import { bridge } from '$lib/bridge';
  import { onMount } from 'svelte';

  let { params }: { params: GradingParams } = $props();

  const wheels = ['lift', 'gamma', 'gain', 'offset'] as const;
  type WheelName = (typeof wheels)[number];
  const channels = ['R', 'G', 'B', 'M'] as const;

  // Canvas refs for each wheel.
  let canvasRefs: Record<string, HTMLCanvasElement | undefined> = $state({});

  const WHEEL_SIZE = 120;
  const WHEEL_RADIUS = WHEEL_SIZE / 2 - 8;
  const RING_WIDTH = 10;

  // Neutral center for each wheel type.
  function neutralRgb(wheel: WheelName): [number, number, number] {
    return wheel === 'gain' ? [1, 1, 1] : [0, 0, 0];
  }

  // Map RGB offset to 2D position (chrominance plane).
  function rgbToXy(r: number, g: number, b: number): [number, number] {
    const x = r - 0.5 * (g + b);
    const y = 0.866 * (g - b);
    return [x, y];
  }

  // Map 2D position back to RGB offset.
  function xyToRgb(x: number, y: number): [number, number, number] {
    const b = -y / 0.866;
    const g = y / 0.866;
    const r = x + 0.5 * (g + b);
    // Solve: x = r - 0.5*(g+b), y = 0.866*(g-b)
    // g - b = y / 0.866
    // r = x + 0.5*(g+b)
    // We need another constraint. Use: r + g + b stays at current sum.
    // Simpler: distribute evenly.
    const halfY = y / (2 * 0.866);
    const dr = x - halfY * 0 + (2 / 3) * x;
    const dg = -(1 / 3) * x + halfY + y / 1.732;
    const db = -(1 / 3) * x - halfY - y / 1.732;
    // Actually use the proper inverse:
    // x = r - 0.5g - 0.5b  → r = x + 0.5g + 0.5b
    // y = 0.866g - 0.866b  → g - b = y/0.866
    // With constraint: average offset = 0 (r + g + b = 0):
    // r + g + b = x + 0.5g + 0.5b + g + b = x + 1.5g + 1.5b = 0
    // g + b = -x/1.5 = -2x/3
    // g - b = y/0.866
    // g = (-2x/3 + y/0.866) / 2 = -x/3 + y/(2*0.866)
    // b = (-2x/3 - y/0.866) / 2 = -x/3 - y/(2*0.866)
    // r = x + 0.5*(-2x/3) = x - x/3 = 2x/3
    const sqrt3 = Math.sqrt(3);
    const rr = (2 / 3) * x;
    const gg = -(1 / 3) * x + y / sqrt3;
    const bb = -(1 / 3) * x - y / sqrt3;
    return [rr, gg, bb];
  }

  // Dragging state.
  let dragWheel: WheelName | null = $state(null);

  function updateWheel(wheel: WheelName, channel: number, value: number) {
    const updated = $state.snapshot(params) as GradingParams;
    updated[wheel][channel] = value;
    bridge.setParams(updated);
  }

  function updateWheelRgb(wheel: WheelName, r: number, g: number, b: number) {
    const updated = $state.snapshot(params) as GradingParams;
    const neutral = neutralRgb(wheel);
    updated[wheel][0] = neutral[0] + r;
    updated[wheel][1] = neutral[1] + g;
    updated[wheel][2] = neutral[2] + b;
    bridge.setParams(updated);
  }

  function handlePointerDown(wheel: WheelName, e: PointerEvent) {
    dragWheel = wheel;
    const target = e.currentTarget as HTMLCanvasElement;
    target.setPointerCapture(e.pointerId);
    applyDrag(wheel, e);
  }

  function handlePointerMove(wheel: WheelName, e: PointerEvent) {
    if (dragWheel !== wheel) return;
    applyDrag(wheel, e);
  }

  function handlePointerUp(wheel: WheelName, _e: PointerEvent) {
    if (dragWheel === wheel) {
      dragWheel = null;
    }
  }

  function applyDrag(wheel: WheelName, e: PointerEvent) {
    const canvas = canvasRefs[wheel];
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const scaleX = WHEEL_SIZE / rect.width;
    const scaleY = WHEEL_SIZE / rect.height;
    const px = (e.clientX - rect.left) * scaleX;
    const py = (e.clientY - rect.top) * scaleY;

    const cx = WHEEL_SIZE / 2;
    const cy = WHEEL_SIZE / 2;
    let dx = px - cx;
    let dy = -(py - cy); // flip Y

    // Clamp to inner radius.
    const innerR = WHEEL_RADIUS - RING_WIDTH - 2;
    const dist = Math.sqrt(dx * dx + dy * dy);
    if (dist > innerR) {
      dx = (dx / dist) * innerR;
      dy = (dy / dist) * innerR;
    }

    // Map pixel offset to colour offset (scale factor).
    const scale = 0.5 / innerR; // ±0.5 range at edge
    const [r, g, b] = xyToRgb(dx * scale, dy * scale);
    updateWheelRgb(wheel, r, g, b);
  }

  // Double-click to reset a wheel to neutral.
  function handleDblClick(wheel: WheelName) {
    const updated = $state.snapshot(params) as GradingParams;
    const neutral = neutralRgb(wheel);
    updated[wheel][0] = neutral[0];
    updated[wheel][1] = neutral[1];
    updated[wheel][2] = neutral[2];
    bridge.setParams(updated);
  }

  // Draw a single colour wheel.
  function drawWheel(wheel: WheelName) {
    const canvas = canvasRefs[wheel];
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const s = WHEEL_SIZE;
    const cx = s / 2;
    const cy = s / 2;
    const outerR = WHEEL_RADIUS;
    const innerR = outerR - RING_WIDTH;

    ctx.clearRect(0, 0, s, s);

    // Dark background circle.
    ctx.fillStyle = '#1a1a1a';
    ctx.beginPath();
    ctx.arc(cx, cy, outerR, 0, Math.PI * 2);
    ctx.fill();

    // Hue ring.
    for (let a = 0; a < 360; a++) {
      const startAngle = ((a - 90) * Math.PI) / 180;
      const endAngle = ((a - 89) * Math.PI) / 180;
      ctx.fillStyle = `hsl(${a}, 60%, 40%)`;
      ctx.beginPath();
      ctx.arc(cx, cy, outerR, startAngle, endAngle);
      ctx.arc(cx, cy, innerR, endAngle, startAngle, true);
      ctx.closePath();
      ctx.fill();
    }

    // Inner disc.
    ctx.fillStyle = '#222';
    ctx.beginPath();
    ctx.arc(cx, cy, innerR - 1, 0, Math.PI * 2);
    ctx.fill();

    // Crosshair.
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(cx, cy - innerR + 4);
    ctx.lineTo(cx, cy + innerR - 4);
    ctx.moveTo(cx - innerR + 4, cy);
    ctx.lineTo(cx + innerR - 4, cy);
    ctx.stroke();

    // Position indicator based on current R, G, B values.
    const neutral = neutralRgb(wheel);
    const dr = params[wheel][0] - neutral[0];
    const dg = params[wheel][1] - neutral[1];
    const db = params[wheel][2] - neutral[2];
    const [px, py] = rgbToXy(dr, dg, db);

    // Scale to canvas coords.
    const scale = (innerR - 4) / 0.5; // ±0.5 range maps to inner radius
    const indicatorX = cx + px * scale;
    const indicatorY = cy - py * scale; // flip Y

    // Indicator dot.
    ctx.fillStyle = '#f28c18';
    ctx.strokeStyle = '#fff';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(indicatorX, indicatorY, 5, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();

    // Center dot.
    ctx.fillStyle = 'rgba(255, 255, 255, 0.3)';
    ctx.beginPath();
    ctx.arc(cx, cy, 2, 0, Math.PI * 2);
    ctx.fill();
  }

  // Redraw all wheels when params change.
  $effect(() => {
    // Touch params to create dependency.
    void params.lift;
    void params.gamma;
    void params.gain;
    void params.offset;
    for (const wheel of wheels) {
      drawWheel(wheel);
    }
  });

  onMount(() => {
    for (const wheel of wheels) {
      drawWheel(wheel);
    }
  });
</script>

<div class="color-wheels">
  <h3>Color Wheels</h3>
  <div class="wheels-grid">
    {#each wheels as wheel}
      <div class="wheel-group">
        <span class="wheel-label">{wheel}</span>
        <div class="wheel-canvas-wrap">
          <canvas
            bind:this={canvasRefs[wheel]}
            width={WHEEL_SIZE}
            height={WHEEL_SIZE}
            onpointerdown={(e) => handlePointerDown(wheel, e)}
            onpointermove={(e) => handlePointerMove(wheel, e)}
            onpointerup={(e) => handlePointerUp(wheel, e)}
            ondblclick={() => handleDblClick(wheel)}
          ></canvas>
        </div>
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
    margin-bottom: 4px;
    text-align: center;
  }

  .wheel-canvas-wrap {
    display: flex;
    justify-content: center;
    margin-bottom: 6px;
  }

  .wheel-canvas-wrap canvas {
    width: 100%;
    max-width: 120px;
    height: auto;
    cursor: crosshair;
    touch-action: none;
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
