<!--
  Dockable panel wrapping the Sliders component.
-->
<script lang="ts">
  import Sliders from '$lib/components/Sliders.svelte';
  import type { GradingParams } from '$lib/types';

  let { params }: { params: GradingParams | null } = $props();
</script>

<div class="svelte-panel">
  {#if params}
    <Sliders {params} />
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
    color: #666;
    text-align: center;
    padding: 20px;
  }
</style>
