# UI Components

## Purpose

Svelte 5 components for the Crispen color grading interface. Each component binds to backend-owned `GradingParams` and sends full parameter updates via the WebSocket bridge on every user interaction.

## Contents

| File | Description |
|------|-------------|
| `ColorWheels.svelte` | Lift/Gamma/Gain/Offset wheel controls — 4 wheels × 4 channels (R/G/B/Master) |
| `PrimaryBars.svelte` | Horizontal bar sliders for Lift/Gamma/Gain/Offset per channel |
| `Sliders.svelte` | Adjustment sliders — temperature, tint, contrast, pivot, saturation, hue, etc. |
| `CurveEditor.svelte` | Tabbed curve editor for Hue-vs-Hue, Hue-vs-Sat, Lum-vs-Sat, Sat-vs-Sat |
| `ScopeDisplay.svelte` | Scope visualizations — histogram (canvas), waveform/vectorscope (info display) |
| `ColorSpaceSelector.svelte` | Dropdown selectors for input, working, and output color spaces |

## Design Decisions

- **Backend-owned state**: Components receive `params` as a prop (read-only from the backend). On change, they `structuredClone(params)`, mutate the clone, and send the full `GradingParams` via `bridge.setParams()`.
- **No optimistic updates**: UI waits for `ParamsUpdated` from backend to reflect changes. This ensures consistency with the single source of truth.
- **Full params on every change**: Each slider/wheel change sends the complete `GradingParams` rather than a delta. Simpler protocol, avoids merge conflicts.

## Dependencies

- **Internal**: `$lib/bridge` (WebSocket IPC), `$lib/types` (TypeScript type definitions)
- **External**: Svelte 5 (runes mode: `$state`, `$props`, `$effect`)

## Usage Examples

```svelte
<script lang="ts">
  import Sliders from '$lib/components/Sliders.svelte';
  let params = $state(/* from bridge */);
</script>

<Sliders {params} />
```
