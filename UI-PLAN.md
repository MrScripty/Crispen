# Native Bevy UI — Image Viewer + Primaries Panel

## Context

All scaffolding phases (0-2) are complete. The app compiles and runs but shows only a blank grey window. The existing WebSocket/Svelte IPC layer exists but is unused — the UI has never been built. We're replacing the webview approach with **native Bevy 0.18 UI** for the primaries panel, starting with:
- Top 2/3: 2D texture viewer showing the graded image
- Bottom 1/3: Primaries panel mimicking DaVinci Resolve's Color page

## Critical Files

### Files to Modify
- `crates/crispen-demo/Cargo.toml` — add `bevy_ui_widgets` feature
- `crates/crispen-demo/src/main.rs` — replace `WebviewPlugin` with `CrispenUiPlugin`, add `UiWidgetsPlugins`

### Files to Create (all in `crates/crispen-demo/src/`)
- `ui/mod.rs` — `CrispenUiPlugin` registration
- `ui/theme.rs` — DaVinci Resolve dark theme constants
- `ui/layout.rs` — Root grid layout (viewer + panel rows)
- `ui/viewer.rs` — Image viewer with dynamic texture updates
- `ui/primaries.rs` — Primaries panel layout: top slider bar, 4 wheels, bottom slider bar
- `ui/color_wheel.rs` — `UiMaterial` color wheel widget + WGSL shader + drag interaction
- `ui/components.rs` — Reusable labeled slider builder, `ParamSlider` marker, `ParamId` enum
- `ui/systems.rs` — Bidirectional sync: sliders/wheels to `GradingParams` and back

### Existing Code to Reuse
- `crates/crispen-bevy/src/resources.rs:16-23` — `GradingState { params, dirty, lut }` is the single source of truth
- `crates/crispen-bevy/src/resources.rs:36-42` — `ImageState { source, graded }` holds graded image for viewer display
- `crates/crispen-bevy/src/systems.rs:101-135` — `rebake_lut_if_dirty` already handles GPU bake when `dirty=true`
- `crates/crispen-bevy/src/events.rs:10-26` — `ColorGradingCommand::SetParams` for sending param updates

### Bevy API References
- `bevy_feathers/controls/color_plane.rs` — Pattern for UiMaterial + pointer drag interaction (our color wheel template)
- `bevy_ui_widgets/slider.rs` — `Slider`, `SliderValue`, `SliderRange`, `SliderThumb`
- `UiWidgetsPlugins` plugin group (includes `SliderPlugin`)
- `UiMaterialPlugin::<T>::default()` for custom shader materials

## Implementation Steps

### Step 1: Cargo.toml — add `bevy_ui_widgets` feature

In `crates/crispen-demo/Cargo.toml`, add `"bevy_ui_widgets"` to the bevy features list. This enables `Slider`, `SliderValue`, `SliderRange`, `SliderThumb`, `UiWidgetsPlugins`.

### Step 2: `ui/theme.rs` — DaVinci Resolve dark theme tokens

Constants for the dark theme matching Resolve's Color page aesthetic:
- Background colors: main `#1E1E1E`, panel `#2D2D2D`, control `#383838`
- Text: primary `#D9D9D9`, dim/labels `#8C8C8C`
- Accent: orange `#F28C18`
- Layout: `WHEEL_SIZE = 140.0`, `SLIDER_HEIGHT = 18.0`, `PANEL_PADDING = 8.0`
- Font size constants for labels, values, section headers

### Step 3: `ui/layout.rs` — root grid

Use `Display::Grid` with two rows:
- Row 0: `GridTrack::fr(2.0)` — image viewer (takes 2/3 of height)
- Row 1: `GridTrack::auto()` — primaries panel (takes remaining space)

Spawn root node, then call `spawn_viewer()` and `spawn_primaries_panel()` as children.

### Step 4: `ui/viewer.rs` — image display with dynamic texture

- **Resource**: `ViewerImageHandle { handle: Handle<Image> }` — holds the Bevy Image asset
- **Startup**: Create a placeholder 1x1 transparent `Image`, spawn an `ImageNode` filling the top grid cell with `object_fit: ObjectFit::Contain` (no stretching)
- **System `update_viewer_texture`**: When `ImageState.graded` changes, convert `GradingImage` (Vec<[f32;4]> linear RGBA) to Bevy `Image` (Rgba8UnormSrgb) by applying sRGB gamma encoding per pixel (`linear_to_srgb()`), then update the `Assets<Image>` entry

### Step 5: `ui/primaries.rs` — panel layout (3 sub-rows)

Layout matches DaVinci Resolve's Primaries panel:

```text
+--------------------------------------------------------------+
|  Temp  |  Tint  |  Contrast |  Pivot  |  Mid Detail          |  <- top slider bar
+--------+--------+-----------+---------+----------------------+
|  LIFT  |  GAMMA |   GAIN    |  OFFSET |  [Master Sliders]    |  <- 4 color wheels
| (wheel)|(wheel) |  (wheel)  | (wheel) |                      |
| R G B M|R G B M |  R G B M  | R G B M |                      |
+--------+--------+-----------+---------+----------------------+
|  Shadows | Highlights | Saturation | Hue | Luma Mix           |  <- bottom slider bar
+--------------------------------------------------------------+
```

- **Top bar**: Horizontal `Display::Flex` row of labeled sliders: Temperature (-100..100), Tint (-100..100), Contrast (0..4), Pivot (0..1), Midtone Detail (-1..1)
- **Wheels row**: 4 columns, each with: color wheel (`MaterialNode<ColorWheelMaterial>`), RGBA value text, label
- **Bottom bar**: Horizontal row of labeled sliders: Shadows (-1..1), Highlights (-1..1), Saturation (0..4), Hue (-180..180), Luma Mix (0..1)

Each slider uses `spawn_param_slider()` from `components.rs`.

### Step 6: `ui/color_wheel.rs` — UiMaterial widget

Follow the `bevy_feathers::ColorPlane` pattern exactly:

**Material:**
```rust
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ColorWheelMaterial {
    #[uniform(0)]
    pub cursor_x: f32,    // normalized -1..1 from center
    #[uniform(0)]
    pub cursor_y: f32,
    #[uniform(0)]
    pub master: f32,      // master channel brightness
}

impl UiMaterial for ColorWheelMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://crispen_demo/shaders/color_wheel.wgsl".into()
    }
}
```

**WGSL shader** (`color_wheel.wgsl`):
- Compute distance from center UV; outer ring (r 0.35..0.5): HSV color wheel with hue=angle, sat=1
- Inner circle (r < 0.35): dimmed background showing current color offset
- Small dot at cursor position
- Anti-aliased edges with `smoothstep()`

**Interaction** (observer-based, matching ColorPlane):
- `Pointer<Press>` + `Pointer<Drag>` + `Pointer<DragEnd>` observers
- Convert pointer position to local coords using `UiGlobalTransform::try_inverse()` + `ComputedNode::size()`
- Map XY to color offset: x = R/G balance, y = B/Y balance (DaVinci Resolve convention)
- Emit `ValueChange<Vec2>` which sync system updates the lift/gamma/gain/offset in `GradingParams`

**Marker component:**
```rust
#[derive(Component)]
pub enum WheelType { Lift, Gamma, Gain, Offset }
```

**Plugin**: `ColorWheelPlugin` registers `UiMaterialPlugin::<ColorWheelMaterial>`, observers, update systems.

### Step 7: `ui/components.rs` — reusable slider builder

```rust
#[derive(Component)]
pub struct ParamSlider(pub ParamId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamId {
    Temperature, Tint, Contrast, Pivot, MidtoneDetail,
    Shadows, Highlights, Saturation, Hue, LumaMix,
}
```

`fn spawn_param_slider(parent, label, param_id, range, default, step)` spawns:
- Vertical container with label text on top
- Bevy `Slider` with `SliderValue(default)`, `SliderRange::new(range.0, range.1)`, styled thumb
- `ParamSlider(param_id)` marker component
- `.observe(slider_self_update)` for auto-value-updating
- Numeric value text below showing current value

### Step 8: `ui/systems.rs` — bidirectional sync

**`sync_sliders_to_params`**: Query `(Entity, &SliderValue, &ParamSlider)` with `Changed<SliderValue>`. Map `ParamId` to the correct `GradingState.params` field, set `dirty = true`.

**`sync_wheels_to_params`**: Listen for `ValueChange<Vec2>` events from wheel entities. Look up `WheelType` on the source entity, map Vec2 to R/G/B channel offsets, update the corresponding `GradingState.params.{lift,gamma,gain,offset}`, set `dirty = true`.

**`sync_params_to_sliders`**: When `GradingState` is changed externally (auto-balance, reset), update all `SliderValue` components to match current params. Use `commands.entity(e).insert(SliderValue(new_val))` (since `SliderValue` is immutable component). Guard against feedback loops by only updating when value differs.

**`sync_params_to_wheels`**: Update `ColorWheelMaterial.cursor_x/cursor_y` when params change externally. Update via `Assets<ColorWheelMaterial>`.

**`update_viewer_texture`**: When `ImageState` changes, convert graded pixels to sRGB u8 and write to viewer Image asset.

### Step 9: `ui/mod.rs` — plugin wiring

```rust
pub struct CrispenUiPlugin;

impl Plugin for CrispenUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ColorWheelPlugin)
            .add_systems(Startup, (setup_viewer, spawn_root_layout).chain())
            .add_systems(Update, (
                sync_sliders_to_params,
                sync_wheels_to_params,
                sync_params_to_sliders,
                sync_params_to_wheels,
                update_viewer_texture,
            ));
    }
}
```

### Step 10: `main.rs` — wire up

- Remove `mod render;` and `use render::WebviewPlugin;`
- Add `mod ui;`
- Replace `.add_plugins(WebviewPlugin)` with `.add_plugins(UiWidgetsPlugins)` + `.add_plugins(ui::CrispenUiPlugin)`
- Keep `CrispenPlugin` (existing ECS + GPU pipeline)
- Keep WebSocket bridge systems for now (they're harmless; can remove later)

## Data Flow

```text
User drags slider
  -> Bevy SliderValue changes
  -> sync_sliders_to_params detects Changed<SliderValue>
  -> Updates GradingState.params.temperature (etc), sets dirty=true
  -> rebake_lut_if_dirty (existing system) bakes LUT on GPU, applies to image
  -> ImageState.graded updated
  -> update_viewer_texture converts to Bevy Image, updates asset
  -> Bevy renders new frame with graded image
```

## Parallel Agent Execution Graph

```text
Phase 0: Setup (sequential, single agent)
-----------------------------------------
  [Scaffolding Agent]
  Cargo.toml feature + ui/mod.rs stub + theme.rs
  + main.rs modifications
  Exit: cargo build succeeds
         |
         v
Phase 1: Widget Development (2 agents in parallel)
-----------------------------------------
  [Agent A: Color Wheel]    ||    [Agent B: Sliders + Viewer]
  - color_wheel.rs          ||    - components.rs (ParamSlider, ParamId)
  - color_wheel.wgsl        ||    - viewer.rs (ViewerImageHandle, texture)
  - WheelType, drag obs.    ||    - theme.rs (fill in constants)
  - ColorWheelPlugin        ||    - spawn_param_slider()
                            ||
         |                  ||           |
         +----------+-------++----------+
                    v
Phase 2: Layout + Integration (sequential, single agent)
-----------------------------------------
  [Integration Agent]
  - layout.rs (root grid)
  - primaries.rs (panel layout using A's wheels + B's sliders)
  - systems.rs (all sync systems)
  - ui/mod.rs (CrispenUiPlugin final wiring)
  Exit: cargo run shows viewer + primaries panel
```

## Verification

1. `cargo build -p crispen-demo` — compiles with no errors
2. `cargo run -p crispen-demo` — window shows:
   - Top 2/3: dark viewer area (placeholder until image loaded)
   - Bottom 1/3: primaries panel with 4 color wheels + slider bars
3. Drag a slider — verify `GradingState.dirty` is set (via tracing debug log)
4. If a test image is loaded, verify slider changes update the displayed image
