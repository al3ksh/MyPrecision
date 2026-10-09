<script lang="ts">
  import Flyout from './views/Flyout.svelte'
  import FullWindow from './views/FullWindow.svelte'

  let { label }: { label: string } = $props()
</script>

{#if label === 'flyout'}
  <Flyout />
{:else}
  <FullWindow />
{/if}
