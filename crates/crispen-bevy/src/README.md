# Bevy Plugin

## Purpose

Integrates the crispen color grading pipeline into Bevy's ECS. Manages grading state as resources, processes UI commands as messages, triggers GPU LUT rebaking on parameter changes, and runs scope computation.

## Contents

| File | Description |
|------|-------------|
| `lib.rs` | `CrispenPlugin` — registers all resources, systems, and GPU startup |
| `resources.rs` | `GradingState`, `ImageState`, `ScopeState`, `ScopeConfig`, `GpuPipelineState` |
| `events.rs` | `ColorGradingCommand` (inbound), `ParamsUpdatedEvent`, `ImageLoadedEvent`, `ScopeDataReadyEvent` (outbound) |
| `systems.rs` | `handle_grading_commands`, `rebake_lut_if_dirty`, `update_scopes`, `detect_param_changes` |
| `render_node.rs` | `GradingRenderNode` — placeholder for render graph integration |
| `scope_render.rs` | `ScopeRenderer` — placeholder for scope texture rendering |

## Design Decisions

- **Message passing**: Uses Bevy 0.18's `Message` type (broadcast) for cross-system communication — no shared mutable state.
- **GPU pipeline as optional resource**: `GpuPipelineState` is inserted by a startup system; systems use `Option<ResMut<...>>` for graceful degradation if GPU is unavailable.
- **CPU scopes from GPU readback**: The GPU bakes + applies the LUT, then reads back the graded image for CPU scope computation. Avoids complex GPU scope readback for now.
- **System ordering**: `handle_grading_commands` → `rebake_lut_if_dirty` → `update_scopes` ensures data flows correctly each frame.

## Dependencies

- **Internal**: `crispen-core` (domain types), `crispen-gpu` (GPU pipeline)
- **External**: `bevy` (ECS framework), `tracing` (logging)

## Usage Examples

```rust
use crispen_bevy::CrispenPlugin;

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(CrispenPlugin)
    .run();
```
