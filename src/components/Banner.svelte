<script lang="ts">
  import { api } from '../lib/api'
  import { BANNER_TEXT, type BannerId } from '../lib/banners'
  import { errorText, toasts } from '../lib/toasts.svelte'

  let { id }: { id: BannerId } = $props()

  const text = $derived(BANNER_TEXT[id])

  // The only banner action; the state-changed event that follows removes the banner.
  function act() {
    api.dismissOptimizerWarning().catch((e) => toasts.push(errorText(e)))
  }
</script>

<div class="banner" class:warning={id === 'optimizer'} role="alert">
  <div class="text">
    <strong>{text.title}</strong>
    <span>{text.body}</span>
  </div>
  {#if text.action}
    <button type="button" onclick={act}>{text.action}</button>
  {/if}
</div>

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    font-size: 12px;
    background: color-mix(in srgb, #ff99a4 14%, transparent);
    border: 1px solid color-mix(in srgb, #ff99a4 35%, var(--border));
    border-radius: var(--r-ctl);
  }

  .banner.warning {
    background: color-mix(in srgb, #fce100 12%, transparent);
    border-color: color-mix(in srgb, #fce100 30%, var(--border));
  }

  .text {
    display: grid;
    gap: 2px;
    flex: 1;
  }

  strong {
    font-weight: 500;
  }

  button {
    flex: none;
    padding: 4px 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-ctl);
    cursor: default;
  }

  button:hover {
    background: var(--surface-hover);
  }
</style>
