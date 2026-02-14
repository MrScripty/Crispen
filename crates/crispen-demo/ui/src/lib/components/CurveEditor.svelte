<script lang="ts">
  import type { GradingParams } from '$lib/types';
  import { bridge } from '$lib/bridge';
  import { onMount } from 'svelte';

  let { params }: { params: GradingParams } = $props();

  type CurveKey = 'hue_vs_hue' | 'hue_vs_sat' | 'lum_vs_sat' | 'sat_vs_sat';

  interface CurveConfig {
    key: CurveKey;
    label: string;
    xLabel: string;
    yLabel: string;
    // Y-axis range for the identity line and display.
    yMin: number;
    yMax: number;
    yIdentity: number; // y-value for no adjustment
  }

  const curves: CurveConfig[] = [
    { key: 'hue_vs_hue', label: 'Hue vs Hue', xLabel: 'Hue', yLabel: 'Shift', yMin: -0.5, yMax: 0.5, yIdentity: 0 },
    { key: 'hue_vs_sat', label: 'Hue vs Sat', xLabel: 'Hue', yLabel: 'Sat', yMin: 0, yMax: 2, yIdentity: 1 },
    { key: 'lum_vs_sat', label: 'Lum vs Sat', xLabel: 'Lum', yLabel: 'Sat', yMin: 0, yMax: 2, yIdentity: 1 },
    { key: 'sat_vs_sat', label: 'Sat vs Sat', xLabel: 'Sat', yLabel: 'Sat', yMin: 0, yMax: 2, yIdentity: 1 },
  ];

  let activeCurve = $state(0);
  let canvas: HTMLCanvasElement | undefined = $state();

  const SIZE = 250;
  const PAD = 24; // padding for axis labels
  const PLOT_SIZE = SIZE - PAD * 2;

  // Dragging state.
  let dragIndex = $state(-1);

  function curveConfig(): CurveConfig {
    return curves[activeCurve];
  }

  function curvePoints(): [number, number][] {
    return params[curveConfig().key] as [number, number][];
  }

  // ── Coordinate mapping ─────────────────────────────────────────

  function dataToCanvas(x: number, y: number): [number, number] {
    const cfg = curveConfig();
    const cx = PAD + x * PLOT_SIZE;
    const cy = PAD + (1 - (y - cfg.yMin) / (cfg.yMax - cfg.yMin)) * PLOT_SIZE;
    return [cx, cy];
  }

  function canvasToData(cx: number, cy: number): [number, number] {
    const cfg = curveConfig();
    const x = Math.max(0, Math.min(1, (cx - PAD) / PLOT_SIZE));
    const yNorm = 1 - (cy - PAD) / PLOT_SIZE;
    const y = cfg.yMin + yNorm * (cfg.yMax - cfg.yMin);
    return [x, Math.max(cfg.yMin, Math.min(cfg.yMax, y))];
  }

  // ── Cubic interpolation ────────────────────────────────────────

  /** Evaluate the curve at a given x using monotone cubic (Catmull-Rom). */
  function evaluateCurve(points: [number, number][], x: number): number {
    const cfg = curveConfig();
    if (points.length === 0) return cfg.yIdentity;
    if (points.length === 1) return points[0][1];

    // Clamp to first/last point.
    if (x <= points[0][0]) return points[0][1];
    if (x >= points[points.length - 1][0]) return points[points.length - 1][1];

    // Find segment.
    let i = 0;
    while (i < points.length - 1 && points[i + 1][0] < x) i++;

    const p0 = points[Math.max(0, i - 1)];
    const p1 = points[i];
    const p2 = points[Math.min(points.length - 1, i + 1)];
    const p3 = points[Math.min(points.length - 1, i + 2)];

    const dx = p2[0] - p1[0];
    if (dx === 0) return p1[1];

    const t = (x - p1[0]) / dx;
    const t2 = t * t;
    const t3 = t2 * t;

    // Catmull-Rom spline.
    const y =
      0.5 * (
        (2 * p1[1]) +
        (-p0[1] + p2[1]) * t +
        (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 +
        (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3
      );

    return Math.max(cfg.yMin, Math.min(cfg.yMax, y));
  }

  // ── Interaction ────────────────────────────────────────────────

  function getCanvasCoords(e: PointerEvent): [number, number] {
    if (!canvas) return [0, 0];
    const rect = canvas.getBoundingClientRect();
    const scaleX = SIZE / rect.width;
    const scaleY = SIZE / rect.height;
    return [(e.clientX - rect.left) * scaleX, (e.clientY - rect.top) * scaleY];
  }

  function findNearestPoint(cx: number, cy: number): number {
    const pts = curvePoints();
    let best = -1;
    let bestDist = 12; // pixel threshold
    for (let i = 0; i < pts.length; i++) {
      const [px, py] = dataToCanvas(pts[i][0], pts[i][1]);
      const d = Math.sqrt((cx - px) ** 2 + (cy - py) ** 2);
      if (d < bestDist) {
        bestDist = d;
        best = i;
      }
    }
    return best;
  }

  function handlePointerDown(e: PointerEvent) {
    if (!canvas) return;
    const [cx, cy] = getCanvasCoords(e);
    const idx = findNearestPoint(cx, cy);

    if (idx >= 0) {
      // Start dragging existing point.
      dragIndex = idx;
      canvas.setPointerCapture(e.pointerId);
    } else {
      // Add a new point.
      const [x, y] = canvasToData(cx, cy);
      const pts = [...curvePoints(), [x, y] as [number, number]];
      pts.sort((a, b) => a[0] - b[0]);
      commitPoints(pts);
    }
  }

  function handlePointerMove(e: PointerEvent) {
    if (dragIndex < 0) return;
    const [cx, cy] = getCanvasCoords(e);
    const [x, y] = canvasToData(cx, cy);
    const pts = [...curvePoints()];
    pts[dragIndex] = [x, y];
    pts.sort((a, b) => a[0] - b[0]);
    // Track which point we're dragging after sort.
    dragIndex = pts.findIndex((p) => p[0] === x && p[1] === y);
    commitPoints(pts);
  }

  function handlePointerUp(_e: PointerEvent) {
    dragIndex = -1;
  }

  function handleDblClick(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const scaleX = SIZE / rect.width;
    const scaleY = SIZE / rect.height;
    const cx = (e.clientX - rect.left) * scaleX;
    const cy = (e.clientY - rect.top) * scaleY;
    const idx = findNearestPoint(cx, cy);
    if (idx >= 0) {
      const pts = [...curvePoints()];
      pts.splice(idx, 1);
      commitPoints(pts);
    }
  }

  function commitPoints(pts: [number, number][]) {
    const updated = $state.snapshot(params) as GradingParams;
    updated[curveConfig().key] = pts;
    bridge.setParams(updated);
  }

  // ── Drawing ────────────────────────────────────────────────────

  function draw() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const cfg = curveConfig();
    const pts = curvePoints();

    ctx.clearRect(0, 0, SIZE, SIZE);
    ctx.fillStyle = '#1a1a1a';
    ctx.fillRect(0, 0, SIZE, SIZE);

    // Grid.
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) {
      const frac = i / 4;
      const cx = PAD + frac * PLOT_SIZE;
      const cy = PAD + frac * PLOT_SIZE;
      ctx.beginPath();
      ctx.moveTo(cx, PAD);
      ctx.lineTo(cx, PAD + PLOT_SIZE);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(PAD, cy);
      ctx.lineTo(PAD + PLOT_SIZE, cy);
      ctx.stroke();
    }

    // Plot border.
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
    ctx.strokeRect(PAD, PAD, PLOT_SIZE, PLOT_SIZE);

    // Identity line (horizontal at yIdentity).
    const [idLeft, idY] = dataToCanvas(0, cfg.yIdentity);
    const [idRight] = dataToCanvas(1, cfg.yIdentity);
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.moveTo(idLeft, idY);
    ctx.lineTo(idRight, idY);
    ctx.stroke();
    ctx.setLineDash([]);

    // Interpolated curve.
    if (pts.length > 0) {
      ctx.strokeStyle = '#f28c18';
      ctx.lineWidth = 2;
      ctx.beginPath();
      const steps = PLOT_SIZE;
      for (let i = 0; i <= steps; i++) {
        const x = i / steps;
        const y = evaluateCurve(pts, x);
        const [cx, cy] = dataToCanvas(x, y);
        if (i === 0) ctx.moveTo(cx, cy);
        else ctx.lineTo(cx, cy);
      }
      ctx.stroke();
      ctx.lineWidth = 1;
    }

    // Control points.
    for (let i = 0; i < pts.length; i++) {
      const [cx, cy] = dataToCanvas(pts[i][0], pts[i][1]);
      ctx.fillStyle = dragIndex === i ? '#fff' : '#f28c18';
      ctx.strokeStyle = '#fff';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.arc(cx, cy, 5, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }

    // Axis labels.
    ctx.fillStyle = '#555';
    ctx.font = '9px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(cfg.xLabel, PAD + PLOT_SIZE / 2, SIZE - 4);
    ctx.save();
    ctx.translate(10, PAD + PLOT_SIZE / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText(cfg.yLabel, 0, 0);
    ctx.restore();

    // Scale labels.
    ctx.fillStyle = '#444';
    ctx.font = '8px sans-serif';
    ctx.textAlign = 'right';
    ctx.fillText(cfg.yMax.toFixed(1), PAD - 4, PAD + 4);
    ctx.fillText(cfg.yMin.toFixed(1), PAD - 4, PAD + PLOT_SIZE + 4);
    ctx.textAlign = 'center';
    ctx.fillText('0', PAD, PAD + PLOT_SIZE + 12);
    ctx.fillText('1', PAD + PLOT_SIZE, PAD + PLOT_SIZE + 12);
  }

  $effect(() => {
    // Reactive dependency on active curve and params.
    void activeCurve;
    void params.hue_vs_hue;
    void params.hue_vs_sat;
    void params.lum_vs_sat;
    void params.sat_vs_sat;
    draw();
  });

  onMount(() => {
    draw();
  });
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
  <div class="curve-canvas-wrap">
    <canvas
      bind:this={canvas}
      width={SIZE}
      height={SIZE}
      onpointerdown={handlePointerDown}
      onpointermove={handlePointerMove}
      onpointerup={handlePointerUp}
      ondblclick={handleDblClick}
    ></canvas>
  </div>
  <p class="curve-hint">Click to add points. Drag to adjust. Double-click to remove.</p>
</div>

<style>
  .curve-editor h3 {
    margin: 0 0 8px;
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

  .curve-canvas-wrap {
    width: 100%;
    aspect-ratio: 1;
    max-width: 250px;
  }

  .curve-canvas-wrap canvas {
    width: 100%;
    height: 100%;
    border-radius: 4px;
    cursor: crosshair;
    touch-action: none;
    display: block;
  }

  .curve-hint {
    font-size: 9px;
    color: #444;
    margin: 4px 0 0;
  }
</style>
