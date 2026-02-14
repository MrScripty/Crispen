<!--
  Dockable panel wrapping the CurveEditor component.
-->
<script lang="ts">
  import CurveEditor from '$lib/components/CurveEditor.svelte';
  import type { GradingParams } from '$lib/types';

  let { params }: { params: GradingParams | null } = $props();
</script>

<div class="svelte-panel">
  {#if params}
    <CurveEditor {params} />
  {:else}
    <p class="placeholder">Waiting for backend...</p>
  {/if}
</div>

<style>
  .svelte-panel {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    pointer-events: auto;
    padding: 8px;
  }

  .placeholder {
    color: var(--color-text-tertiary);
    text-align: center;
    padding: 20px;
  }
</style>
