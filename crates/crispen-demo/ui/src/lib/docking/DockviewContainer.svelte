<!--
  Dockview container — creates and manages the dockview layout.

  Uses dockview-core directly (framework-agnostic) with Svelte 5 mount/unmount
  to render panel contents inside dockview's DOM elements.
-->
<script lang="ts">
  import { onMount, onDestroy, mount, unmount } from 'svelte';
  import { createDockview } from 'dockview-core';
  import type {
    DockviewApi,
    IContentRenderer,
    GroupPanelPartInitParameters,
  } from 'dockview-core';
  import 'dockview-core/dist/styles/dockview.css';
  import './dockview-theme.css';

  import { bridge } from '$lib/bridge';
  import type {
    GradingParams,
    LayoutRegion,
  } from '$lib/types';

  // Panel components
  import BevyPanel from './panels/BevyPanel.svelte';
  import SlidersPanel from './panels/SlidersPanel.svelte';
  import PrimaryBarsPanel from './panels/PrimaryBarsPanel.svelte';
  import CurvesPanel from './panels/CurvesPanel.svelte';
  import ColorWheelsPanel from './panels/ColorWheelsPanel.svelte';

  let {
    params,
  }: {
    params: GradingParams | null;
  } = $props();

  // Reactive state objects for imperatively mounted panels.
  // Svelte 5's mount() requires $state objects for props to stay reactive.
  const paramProps = $state({ params: null as GradingParams | null });

  // Sync incoming props to $state objects so mounted panels update reactively.
  $effect(() => { paramProps.params = params; });

  let containerEl: HTMLDivElement | undefined = $state();
  let api: DockviewApi | undefined = $state();
  let disposables: Array<{ dispose: () => void }> = [];

  // Bevy panel IDs — these are transparent cutouts positioned by Bevy
  const BEVY_PANELS = ['viewer', 'scopes'];

  // Map component names to Svelte component constructors and their props.
  // Static panels get plain objects; reactive panels get $state objects.
  type PanelFactory = {
    component: any;
    getProps: () => Record<string, unknown>;
  };

  function getPanelFactories(): Record<string, PanelFactory> {
    return {
      viewer: {
        component: BevyPanel,
        getProps: () => ({ panelId: 'viewer' }),
      },
      sliders: {
        component: SlidersPanel,
        getProps: () => paramProps,
      },
      'primary-bars': {
        component: PrimaryBarsPanel,
        getProps: () => paramProps,
      },
      curves: {
        component: CurvesPanel,
        getProps: () => paramProps,
      },
      scopes: {
        component: BevyPanel,
        getProps: () => ({ panelId: 'scopes' }),
      },
      'color-wheels': {
        component: ColorWheelsPanel,
        getProps: () => paramProps,
      },
    };
  }

  /** Creates a dockview IContentRenderer that mounts a Svelte component. */
  function createSvelteRenderer(componentName: string): IContentRenderer {
    const element = document.createElement('div');
    element.style.width = '100%';
    element.style.height = '100%';
    element.style.overflow = 'hidden';

    let svelteInstance: Record<string, unknown> | null = null;

    return {
      element,
      init(_params: GroupPanelPartInitParameters) {
        const factories = getPanelFactories();
        const factory = factories[componentName];
        if (!factory) {
          element.textContent = `Unknown panel: ${componentName}`;
          return;
        }
        svelteInstance = mount(factory.component, {
          target: element,
          props: factory.getProps(),
        });
      },
      dispose() {
        if (svelteInstance) {
          unmount(svelteInstance);
          svelteInstance = null;
        }
      },
    };
  }

  /** Collect all Bevy panel regions from the DOM. */
  function collectBevyRegions(): LayoutRegion[] {
    const regions: LayoutRegion[] = [];
    if (!api) return regions;

    for (const panelId of BEVY_PANELS) {
      const panel = api.getPanel(panelId);
      if (!panel) continue;

      const el = panel.view?.content?.element;
      if (!el) continue;

      const rect = el.getBoundingClientRect();
      regions.push({
        id: panelId,
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        visible: panel.api.isVisible,
      });
    }

    return regions;
  }

  /** Send all Bevy panel positions to Bevy for native widget positioning. */
  function syncLayoutToBevy() {
    const regions = collectBevyRegions();
    if (regions.length > 0) {
      bridge.updateLayout(regions);
    }
  }

  function setupDefaultLayout(dockviewApi: DockviewApi) {
    // Left column: scopes
    dockviewApi.addPanel({
      id: 'scopes',
      component: 'scopes',
      title: 'Scopes',
      initialWidth: 320,
    });

    // Center: viewer
    dockviewApi.addPanel({
      id: 'viewer',
      component: 'viewer',
      title: 'Viewer',
      position: { referencePanel: 'scopes', direction: 'right' },
    });

    // Bottom-left: color wheels
    dockviewApi.addPanel({
      id: 'color-wheels',
      component: 'color-wheels',
      title: 'Color Wheels',
      position: { referencePanel: 'viewer', direction: 'below' },
      initialHeight: 280,
    });

    // Bottom-center: sliders (tabbed with primary bars)
    dockviewApi.addPanel({
      id: 'sliders',
      component: 'sliders',
      title: 'Sliders',
      position: { referencePanel: 'color-wheels', direction: 'right' },
    });

    dockviewApi.addPanel({
      id: 'primary-bars',
      component: 'primary-bars',
      title: 'Primary Bars',
      position: { referencePanel: 'sliders', direction: 'within' },
    });

    // Bottom-right: curves
    dockviewApi.addPanel({
      id: 'curves',
      component: 'curves',
      title: 'Curves',
      position: { referencePanel: 'sliders', direction: 'right' },
    });
  }

  onMount(() => {
    if (!containerEl) return;

    const dockviewApi = createDockview(containerEl, {
      createComponent(options) {
        return createSvelteRenderer(options.name);
      },
      className: 'crispen-dockview',
    });

    api = dockviewApi;

    // Try to restore saved layout, otherwise use defaults
    const saved = localStorage.getItem('crispen-layout');
    if (saved) {
      try {
        dockviewApi.fromJSON(JSON.parse(saved));
      } catch {
        setupDefaultLayout(dockviewApi);
      }
    } else {
      setupDefaultLayout(dockviewApi);
    }

    // Listen for layout changes → sync to Bevy + auto-save
    let saveTimer: ReturnType<typeof setTimeout> | null = null;

    disposables.push(
      dockviewApi.onDidLayoutChange(() => {
        syncLayoutToBevy();

        // Debounced auto-save (1s)
        if (saveTimer) clearTimeout(saveTimer);
        saveTimer = setTimeout(() => {
          const json = JSON.stringify(dockviewApi.toJSON());
          localStorage.setItem('crispen-layout', json);
          bridge.saveLayout(json);
        }, 1000);
      }),
    );

    disposables.push(
      dockviewApi.onDidAddPanel(() => syncLayoutToBevy()),
    );

    disposables.push(
      dockviewApi.onDidRemovePanel(() => syncLayoutToBevy()),
    );

    // Re-sync layout when the CEF IPC bridge becomes available.
    // Early syncLayoutToBevy() calls may have been queued before the bridge
    // was injected — this ensures Bevy receives the panel regions.
    const onIpcReady = () => syncLayoutToBevy();
    window.addEventListener('crispen-ipc-ready', onIpcReady);
    disposables.push({ dispose: () => window.removeEventListener('crispen-ipc-ready', onIpcReady) });

    // Initial sync
    syncLayoutToBevy();
  });

  onDestroy(() => {
    for (const d of disposables) {
      d.dispose();
    }
    disposables = [];

    if (api) {
      api.dispose();
      api = undefined;
    }
  });
</script>

<div class="dockview-wrapper" bind:this={containerEl}></div>

<style>
  .dockview-wrapper {
    flex: 1;
    width: 100%;
    overflow: hidden;
  }
</style>
