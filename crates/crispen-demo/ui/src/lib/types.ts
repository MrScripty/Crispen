/**
 * TypeScript types matching crispen-demo/src/ipc.rs exactly.
 *
 * These types mirror the Rust `BevyToUi` / `UiToBevy` enums and all
 * domain types they reference. The serde serialization uses
 * `#[serde(tag = "type", content = "data")]`.
 */

// -- Domain types --

export interface ColorManagementConfig {
  input_space: string;
  working_space: string;
  output_space: string;
  display_oetf: string;
}

export interface GradingParams {
  color_management: ColorManagementConfig;
  lift: [number, number, number, number];
  gamma: [number, number, number, number];
  gain: [number, number, number, number];
  offset: [number, number, number, number];
  lift_wheel: [number, number, number, number];
  gamma_wheel: [number, number, number, number];
  gain_wheel: [number, number, number, number];
  offset_wheel: [number, number, number, number];
  temperature: number;
  tint: number;
  contrast: number;
  pivot: number;
  midtone_detail: number;
  shadows: number;
  highlights: number;
  saturation: number;
  hue: number;
  luma_mix: number;
  hue_vs_hue: [number, number][];
  hue_vs_sat: [number, number][];
  lum_vs_sat: [number, number][];
  sat_vs_sat: [number, number][];
}

// -- Scope data --

export interface HistogramData {
  bins: number[][];
  peak: number;
}

export interface WaveformData {
  width: number;
  height: number;
  data: number[][];
}

export interface VectorscopeData {
  resolution: number;
  density: number[];
}

export interface CieData {
  resolution: number;
  density: number[];
}

// -- Layout --

export interface LayoutRegion {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  visible: boolean;
}

// -- IPC messages (tag + content pattern) --

export type BevyToUi =
  | { type: 'Initialize'; data: { params: GradingParams } }
  | { type: 'ParamsUpdated'; data: { params: GradingParams } }
  | {
      type: 'ScopeData';
      data: {
        histogram: HistogramData;
        waveform: WaveformData;
        vectorscope: VectorscopeData;
        cie: CieData;
      };
    }
  | { type: 'ImageLoaded'; data: { path: string; width: number; height: number; bit_depth: string } }
  | { type: 'Error'; data: { message: string } };

export type UiToBevy =
  | { type: 'RequestState' }
  | { type: 'SetParams'; data: { params: GradingParams } }
  | { type: 'AutoBalance' }
  | { type: 'ResetGrade' }
  | { type: 'LoadImage'; data: { path: string } }
  | { type: 'LoadLut'; data: { path: string; slot: string } }
  | { type: 'ExportLut'; data: { path: string; size: number } }
  | { type: 'ToggleScope'; data: { scope_type: string; visible: boolean } }
  | { type: 'UiDirty' }
  | { type: 'LayoutUpdate'; data: { regions: LayoutRegion[] } }
  | { type: 'SaveLayout'; data: { layout_json: string } };
