<script lang="ts">
  import { toasts } from '../lib/toasts.svelte'

  /** `floating` pins the stack to the window corner (full window); otherwise it flows inline. */
  let { floating = false }: { floating?: boolean } = $props()
</script>

<div class="toasts" class:floating role="status">
  {#each toasts.items as toast (toast.id)}
    <p class="toast">{toast.text}</p>
  {/each}
</div>

<style>
  .toasts {
    display: grid;
    gap: 6px;
  }

  .toasts:empty {
    display: none;
  }

  .floating {
    position: fixed;
    right: 20px;
    bottom: 20px;
    max-width: 360px;
  }

  .toast {
    margin: 0;
    padding: 8px 12px;
    font-size: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-left: 3px solid #ff99a4;
    border-radius: var(--r-ctl);
    animation: in 150ms var(--ease) both;
  }

  @keyframes in {
    from {
      transform: translateY(4px);
      opacity: 0;
    }
  }
</style>
