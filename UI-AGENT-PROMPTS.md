# Crispen UI — Agent Prompts

Detailed prompts for each agent phase. See `UI-PLAN.md` for the full architecture.

## Reference Paths

- **Crispen repo:** `/media/jeremy/OrangeCream/Linux Software/Crispen/`
- **UI Plan:** `/media/jeremy/OrangeCream/Linux Software/Crispen/UI-PLAN.md`
- **Coding Standards:** `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/`
- **Bevy source:** `/media/jeremy/OrangeCream/Linux Software/bevy/`
- **Pentimento (sister project):** `/media/jeremy/OrangeCream/Linux Software/Pentimento/`

---

## Phase 0 Prompt: Scaffolding Agent

```
You are setting up the native Bevy UI module for Crispen's demo application.

## Your Task

Create the module structure, update Cargo.toml, modify main.rs, and create
the theme file so that Agents A and B can work in parallel on widgets.

## Read First

1. /media/jeremy/OrangeCream/Linux Software/Crispen/PLAN.md — full project plan
2. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/src/main.rs — current app entry
3. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/Cargo.toml — current deps
4. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/CODING-STANDARDS.md — Rust guidelines

## Steps

1. In crates/crispen-demo/Cargo.toml, add "bevy_ui_widgets" to bevy features list.

2. Create crates/crispen-demo/src/ui/mod.rs:
   - pub mod theme, layout, viewer, primaries, color_wheel, components, systems
   - pub struct CrispenUiPlugin; with empty Plugin impl (systems added in Phase 2)

3. Create crates/crispen-demo/src/ui/theme.rs:
   - DaVinci Resolve dark theme constants (Color values, sizing, spacing)
   - BG_DARK, BG_PANEL, BG_CONTROL, TEXT_PRIMARY, TEXT_DIM, ACCENT
   - WHEEL_SIZE, SLIDER_HEIGHT, PANEL_PADDING, FONT_SIZE_LABEL, FONT_SIZE_VALUE

4. Modify crates/crispen-demo/src/main.rs:
   - Add "mod ui;"
   - Remove "use render::WebviewPlugin;"
   - Replace .add_plugins(WebviewPlugin) with:
     .add_plugins(bevy::ui_widgets::UiWidgetsPlugins)
     .add_plugins(ui::CrispenUiPlugin)
   - Keep CrispenPlugin, camera, and WebSocket systems

5. Create stub files (empty pub mod or todo!() bodies):
   - ui/layout.rs, ui/viewer.rs, ui/primaries.rs
   - ui/color_wheel.rs, ui/components.rs, ui/systems.rs

6. Verify: cargo build -p crispen-demo succeeds.

## Commit

feat(demo): scaffold native Bevy UI module structure

Agent: ui-scaffolding-agent
```

---

## Phase 1, Agent A Prompt: Color Wheel Widget

```
You are implementing the color wheel widget for Crispen's DaVinci Resolve-style
primaries panel. This is a self-contained UiMaterial-based 2D control.

## Your Task

Create a reusable color wheel widget using Bevy's UiMaterial system with a
custom WGSL fragment shader and pointer-based drag interaction. Follow the
bevy_feathers ColorPlane pattern exactly.

## Read First

1. /media/jeremy/OrangeCream/Linux Software/Crispen/UI-PLAN.md — Step 6 (color wheel spec)
2. /media/jeremy/OrangeCream/Linux Software/bevy/crates/bevy_feathers/src/controls/color_plane.rs
   — YOUR TEMPLATE. Study the entire file: UiMaterial impl, Pointer<Press/Drag/DragEnd>
   observers, UiGlobalTransform coordinate conversion, ValueChange<Vec2> emission.
3. /media/jeremy/OrangeCream/Linux Software/bevy/crates/bevy_feathers/src/assets/shaders/color_plane.wgsl
   — Reference WGSL shader for UiMaterial
4. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/CODING-STANDARDS.md

## Scope — Files You Own

Only modify files in /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/src/ui/

- ui/color_wheel.rs (main implementation)
- Create WGSL shader file (embedded or asset)

## Implementation

### color_wheel.rs

1. Define ColorWheelMaterial:
   #[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
   pub struct ColorWheelMaterial {
       #[uniform(0)]
       pub cursor_x: f32,      // -1..1 from center
       #[uniform(0)]
       pub cursor_y: f32,      // -1..1 from center
       #[uniform(0)]
       pub master: f32,        // master channel brightness overlay
   }

   impl UiMaterial for ColorWheelMaterial {
       fn fragment_shader() -> ShaderRef { ... }
   }

2. Define WheelType marker component:
   #[derive(Component, Debug, Clone, Copy)]
   pub enum WheelType { Lift, Gamma, Gain, Offset }

3. Define ColorWheelDragState (matches ColorPlaneDragState pattern):
   #[derive(Component, Default)]
   struct ColorWheelDragState(bool);

4. Define ColorWheelInner marker (inner node that gets MaterialNode)
   and ColorWheelThumb marker (positioned dot).

5. pub fn color_wheel(wheel_type: WheelType) -> impl Bundle
   Match the color_plane() template function:
   - Outer container Node with fixed size (WHEEL_SIZE x WHEEL_SIZE)
   - WheelType component
   - ColorWheelDragState
   - Inner child with ColorWheelInner marker
   - Thumb child: small circle (10x10) with border, positioned absolutely,
     Pickable::IGNORE, UiTransform offset -50% for centering
   - Label text child below the wheel

6. Implement observers (copy ColorPlane pattern exactly):
   - on_pointer_press: compute local position, normalize to -1..1, emit ValueChange<Vec2>
   - on_drag_start: set drag state
   - on_drag: compute position, emit ValueChange<Vec2>
   - on_drag_end: clear drag state
   - on_drag_cancel: clear drag state

7. System: update_wheel_material
   When WheelType entity's associated ColorWheelMaterial needs updating
   (from external param changes), update cursor_x/cursor_y and master.

8. System: update_wheel_thumb
   When ValueChange<Vec2> is received, update thumb position to
   Val::Percent((value.x * 0.5 + 0.5) * 100.0) etc.

9. pub struct ColorWheelPlugin — registers:
   - UiMaterialPlugin::<ColorWheelMaterial>::default()
   - PostUpdate system: update_wheel_material
   - Observers: on_pointer_press, on_drag_start, on_drag, on_drag_end, on_drag_cancel

### WGSL Shader (color_wheel.wgsl)

Fragment shader that receives UVs from Bevy's UI material pipeline:

- Input: @location(0) uv: vec2<f32>, plus uniform with cursor_x, cursor_y, master
- Compute centered coords: let p = uv - vec2(0.5); let r = length(p);
- Outer ring (0.35 < r < 0.50): HSV wheel
  - angle = atan2(p.y, p.x) -> hue (0..2pi -> 0..1)
  - HSV to RGB conversion
  - Alpha = smoothstep for anti-aliased edge
- Inner circle (r < 0.33):
  - Dark background (0.15, 0.15, 0.15) mixed with slight color from cursor position
  - Alpha = smoothstep for anti-aliased edge
- Cursor dot:
  - let cursor_pos = vec2(cursor_x, cursor_y) * 0.3; (scale to inner circle)
  - let dot_dist = length(p - cursor_pos);
  - if dot_dist < 0.02: white dot with anti-aliased edge
- Outside circle (r > 0.50): discard (alpha = 0)

## Commits

feat(demo): implement color wheel UiMaterial widget with drag interaction

Agent: color-wheel-agent

## Rules

- Follow bevy_feathers ColorPlane pattern for interaction (observers, not polling)
- Use ValueChange<Vec2> from bevy_ui_widgets for output events
- The widget must be self-contained — no dependency on other ui/ modules except theme.rs
- All pointer coordinate math uses UiGlobalTransform::try_inverse() + ComputedNode::size()
- Keep under 500 lines; split shader to separate file
```

---

## Phase 1, Agent B Prompt: Sliders + Viewer

```
You are implementing the slider components and image viewer for Crispen's
native Bevy UI. These are self-contained widgets used by the primaries panel.

## Your Task

Create the reusable labeled slider builder (ParamSlider), the ParamId enum,
and the image viewer with dynamic texture updates. Fill in theme constants.

## Read First

1. /media/jeremy/OrangeCream/Linux Software/Crispen/UI-PLAN.md — Steps 2, 4, 7
2. /media/jeremy/OrangeCream/Linux Software/bevy/crates/bevy_ui_widgets/src/slider.rs
   — Slider API: SliderValue(f32), SliderRange::new(), SliderThumb, TrackClick,
   slider_self_update observer, SliderStep, SliderPrecision
3. /media/jeremy/OrangeCream/Linux Software/bevy/examples/ui/vertical_slider.rs
   — Full slider setup example with track + thumb children
4. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-bevy/src/resources.rs
   — ImageState { source, graded } and GradingState
5. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-core/src/image.rs
   — GradingImage struct (width, height, pixels: Vec<[f32; 4]>)
6. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/CODING-STANDARDS.md

## Scope — Files You Own

Only modify files in /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/src/ui/

- ui/components.rs
- ui/viewer.rs
- ui/theme.rs (fill in constants if still stubs)

## Implementation

### ui/theme.rs (complete the constants)

pub const BG_DARK: Color = Color::srgb(0.118, 0.118, 0.118);
pub const BG_PANEL: Color = Color::srgb(0.176, 0.176, 0.176);
pub const BG_CONTROL: Color = Color::srgb(0.22, 0.22, 0.22);
pub const TEXT_PRIMARY: Color = Color::srgb(0.85, 0.85, 0.85);
pub const TEXT_DIM: Color = Color::srgb(0.55, 0.55, 0.55);
pub const ACCENT: Color = Color::srgb(0.95, 0.55, 0.094);
pub const SLIDER_TRACK: Color = Color::srgb(0.25, 0.25, 0.25);
pub const SLIDER_THUMB: Color = Color::srgb(0.7, 0.7, 0.7);
pub const WHEEL_SIZE: f32 = 140.0;
pub const SLIDER_HEIGHT: f32 = 18.0;
pub const PANEL_PADDING: f32 = 8.0;
pub const FONT_SIZE_LABEL: f32 = 11.0;
pub const FONT_SIZE_VALUE: f32 = 10.0;

### ui/components.rs

1. Define ParamId enum:
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
   pub enum ParamId {
       Temperature, Tint, Contrast, Pivot, MidtoneDetail,
       Shadows, Highlights, Saturation, Hue, LumaMix,
   }

2. Define ParamSlider marker:
   #[derive(Component)]
   pub struct ParamSlider(pub ParamId);

3. Define ValueLabel marker (entity holding the numeric text display):
   #[derive(Component)]
   pub struct ParamValueLabel(pub Entity);  // points to slider entity

4. pub fn spawn_param_slider(
       parent: &mut ChildSpawnerCommands,
       label: &str,
       param_id: ParamId,
       range: (f32, f32),
       default_val: f32,
       step: f32,
   )
   Spawns a vertical container:
   - Text label (FONT_SIZE_LABEL, TEXT_DIM)
   - Horizontal Slider node:
     - width: 80px (or flex grow), height: SLIDER_HEIGHT
     - Slider { track_click: TrackClick::Snap }
     - SliderValue(default_val)
     - SliderRange::new(range.0, range.1)
     - SliderStep(step)
     - ParamSlider(param_id) marker
     - .observe(slider_self_update) for auto-update
     - Track child: thin bar (4px height, SLIDER_TRACK color)
     - Thumb child: small rect (8x14), SLIDER_THUMB color, SliderThumb marker
   - Text value display (FONT_SIZE_VALUE, TEXT_PRIMARY), formatted "{:.2}"
     with ParamValueLabel marker pointing to slider entity

5. pub fn param_default(id: ParamId) -> f32
   Returns the GradingParams default for each param:
   Temperature/Tint/MidtoneDetail/Shadows/Highlights/Hue/LumaMix -> 0.0
   Contrast/Saturation -> 1.0
   Pivot -> 0.435

6. pub fn param_range(id: ParamId) -> (f32, f32)
   Returns the slider range for each param.

### ui/viewer.rs

1. Resource:
   #[derive(Resource)]
   pub struct ViewerImageHandle { pub handle: Handle<Image> }

2. pub fn setup_viewer(
       mut commands: Commands,
       mut images: ResMut<Assets<Image>>,
   )
   - Create 1x1 transparent Image (Rgba8UnormSrgb)
   - Add to Assets, store handle in ViewerImageHandle resource
   - (The actual ImageNode is spawned by layout.rs, which calls viewer functions)

3. pub fn spawn_viewer_node(parent: &mut ChildSpawnerCommands, handle: Handle<Image>)
   - Spawn ImageNode { image: handle } with:
     - width: Val::Percent(100.0)
     - height: Val::Percent(100.0)
     - background_color: BG_DARK
     - overflow: hidden (clip to container)

4. pub fn update_viewer_texture(
       image_state: Res<ImageState>,
       viewer: Res<ViewerImageHandle>,
       mut images: ResMut<Assets<Image>>,
   )
   - Guard: if !image_state.is_changed() { return; }
   - Guard: if graded is None, return
   - Convert GradingImage pixels to u8 sRGB:
     fn linear_to_srgb(c: f32) -> u8 {
         let s = if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0/2.4) - 0.055 };
         (s.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
     }
   - Create new Image with correct dimensions, Rgba8UnormSrgb format
   - Replace the asset entry: *images.get_mut(&viewer.handle).unwrap() = new_image;

## Commits

feat(demo): implement param slider components and image viewer

Agent: slider-viewer-agent

## Rules

- Sliders use bevy_ui_widgets Slider API with .observe(slider_self_update)
- ParamSlider marker enables the sync system (Phase 2) to map values to GradingParams
- Viewer converts f32 linear -> u8 sRGB — the standard gamma encoding
- No dependency on color_wheel.rs (Agent A's work)
- Keep under 500 lines per file
```

---

## Phase 2 Prompt: Integration Agent

```
You are wiring the color wheel and slider widgets into the full primaries
panel layout, creating the sync systems, and connecting everything to the
existing CrispenPlugin grading pipeline.

## Your Task

Create the panel layout, root grid, all bidirectional sync systems, and
finalize CrispenUiPlugin. After this, the app should display a viewer and
functional primaries panel.

## Read First

1. /media/jeremy/OrangeCream/Linux Software/Crispen/UI-PLAN.md — Steps 3, 5, 8, 9, 10
2. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/src/ui/color_wheel.rs — Agent A output
3. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/src/ui/components.rs — Agent B output
4. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/src/ui/viewer.rs — Agent B output
5. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-bevy/src/resources.rs — GradingState, ImageState
6. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-bevy/src/events.rs — ColorGradingCommand
7. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-bevy/src/systems.rs — existing rebake/scope systems

## Scope — Files You Own

- /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/src/ui/layout.rs
- /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/src/ui/primaries.rs
- /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/src/ui/systems.rs
- /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/src/ui/mod.rs (finalize)

## Implementation

### ui/layout.rs

pub fn spawn_root_layout(
    mut commands: Commands,
    viewer_handle: Res<ViewerImageHandle>,
)
- Spawn root Node with Display::Grid:
  - width: Val::Percent(100.0), height: Val::Percent(100.0)
  - grid_template_rows: vec![GridTrack::fr(2.0), GridTrack::auto()]
  - grid_template_columns: vec![GridTrack::fr(1.0)]
  - background_color: BG_DARK
- Row 0: call viewer::spawn_viewer_node(parent, viewer_handle.handle.clone())
- Row 1: call primaries::spawn_primaries_panel(parent)

### ui/primaries.rs

pub fn spawn_primaries_panel(parent: &mut ChildSpawnerCommands)

Panel container (BG_PANEL background, padding PANEL_PADDING):
  Display::Flex, flex_direction: Column

  1. Top slider bar (Display::Flex, Row, gap: 12px):
     spawn_param_slider for: Temperature, Tint, Contrast, Pivot, MidtoneDetail

  2. Wheels row (Display::Flex, Row, justify: SpaceEvenly):
     For each of [Lift, Gamma, Gain, Offset]:
       Vertical column containing:
         - color_wheel(wheel_type) from color_wheel.rs
         - Row of 4 small value texts: R, G, B, Master
         - Label text: "LIFT" / "GAMMA" / "GAIN" / "OFFSET"

  3. Bottom slider bar (Display::Flex, Row, gap: 12px):
     spawn_param_slider for: Shadows, Highlights, Saturation, Hue, LumaMix

### ui/systems.rs

1. sync_sliders_to_params(
       sliders: Query<(&SliderValue, &ParamSlider), Changed<SliderValue>>,
       mut state: ResMut<GradingState>,
   )
   For each changed slider, match ParamId to GradingState.params field:
     ParamId::Temperature => state.params.temperature = value.0,
     etc.
   Set state.dirty = true.

2. sync_wheels_to_params — observe ValueChange<Vec2> trigger:
   fn on_wheel_value_change(
       trigger: Trigger<ValueChange<Vec2>>,
       wheels: Query<&WheelType>,
       mut state: ResMut<GradingState>,
   )
   Map Vec2 to channel adjustments:
   - x maps to R-G balance (positive = more red, negative = more green)
   - y maps to B balance (positive = more blue, negative = more yellow)
   - Derive [R, G, B, master] from Vec2
   Match WheelType to lift/gamma/gain/offset field, update, set dirty.

3. sync_params_to_sliders(
       state: Res<GradingState>,
       mut sliders: Query<(Entity, &ParamSlider, &SliderValue)>,
       mut commands: Commands,
   )
   Guard: if !state.is_changed() { return; }
   For each slider, read the matching param value. If it differs from
   current SliderValue, insert new SliderValue via commands.

4. sync_params_to_wheels(
       state: Res<GradingState>,
       wheels: Query<(Entity, &WheelType)>,
       ... access to ColorWheelMaterial assets ...
   )
   Guard: if !state.is_changed() { return; }
   For each wheel, derive cursor_x/cursor_y from the current lift/gamma/gain/offset
   values and update the material asset.

5. update_viewer_texture — already implemented in viewer.rs, just register here.

6. update_value_labels(
       sliders: Query<(&SliderValue, &ParamValueLabel), Changed<SliderValue>>,
       mut texts: Query<&mut Text>,
   )
   Update numeric text displays when slider values change.

### ui/mod.rs (finalize)

pub struct CrispenUiPlugin;

impl Plugin for CrispenUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ColorWheelPlugin)
            .add_systems(Startup, (
                viewer::setup_viewer,
                layout::spawn_root_layout,
            ).chain())
            .add_systems(Update, (
                systems::sync_sliders_to_params,
                systems::sync_params_to_sliders,
                systems::sync_params_to_wheels,
                systems::update_value_labels,
                viewer::update_viewer_texture,
            ))
            .add_observer(systems::on_wheel_value_change);
    }
}

## Verification

1. cargo build -p crispen-demo — zero errors
2. cargo run -p crispen-demo:
   - Top 2/3: dark viewer area
   - Bottom 1/3: primaries panel with 4 wheels + sliders
3. Drag a slider -> tracing log shows "GradingState changed, dirty=true"
4. Drag a color wheel -> same dirty=true log

## Commits

feat(demo): implement primaries panel layout with wheels and sliders
feat(demo): implement bidirectional param sync systems
feat(demo): finalize CrispenUiPlugin wiring

Agent: ui-integration-agent
```
