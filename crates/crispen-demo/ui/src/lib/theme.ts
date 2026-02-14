/**
 * Read a CSS custom property value from :root.
 *
 * Canvas 2D contexts cannot use `var(--…)` directly, so we resolve
 * them here once and pass the concrete strings to drawing code.
 */
function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

/** All canvas-relevant theme colors, resolved once and cached. */
export interface CanvasTheme {
  bgCanvas: string;
  bgCanvasDeep: string;
  bgInnerDisc: string;
  grid: string;
  crosshair: string;
  guide: string;
  center: string;
  identity: string;
  accent: string;
  textDim: string;
  textHint: string;
  textTitle: string;
  channelR60: string;
  channelG60: string;
  channelB60: string;
  scopeSkinTone: string;
  scopeTargetR: string;
  scopeTargetYl: string;
  scopeTargetG: string;
  scopeTargetCy: string;
  scopeTargetB: string;
  scopeTargetMg: string;
}

let cached: CanvasTheme | null = null;

/** Resolve all canvas theme tokens from CSS custom properties. */
export function getCanvasTheme(): CanvasTheme {
  if (cached) return cached;
  cached = {
    bgCanvas: cssVar('--color-bg-canvas'),
    bgCanvasDeep: cssVar('--color-bg-canvas-deep'),
    bgInnerDisc: cssVar('--color-bg-inner-disc'),
    grid: cssVar('--color-canvas-grid'),
    crosshair: cssVar('--color-canvas-crosshair'),
    guide: cssVar('--color-canvas-guide'),
    center: cssVar('--color-canvas-center'),
    identity: cssVar('--color-canvas-identity'),
    accent: cssVar('--color-accent'),
    textDim: cssVar('--color-text-dim'),
    textHint: cssVar('--color-text-hint'),
    textTitle: cssVar('--color-text-title'),
    channelR60: cssVar('--color-channel-r-60'),
    channelG60: cssVar('--color-channel-g-60'),
    channelB60: cssVar('--color-channel-b-60'),
    scopeSkinTone: cssVar('--color-scope-skin-tone'),
    scopeTargetR: cssVar('--color-scope-target-r'),
    scopeTargetYl: cssVar('--color-scope-target-yl'),
    scopeTargetG: cssVar('--color-scope-target-g'),
    scopeTargetCy: cssVar('--color-scope-target-cy'),
    scopeTargetB: cssVar('--color-scope-target-b'),
    scopeTargetMg: cssVar('--color-scope-target-mg'),
  };
  return cached;
}

/** Force re-read on next access (call if the theme ever changes at runtime). */
export function invalidateCanvasTheme(): void {
  cached = null;
}
