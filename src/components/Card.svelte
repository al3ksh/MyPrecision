<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    title?: string
    children: Snippet
    actions?: Snippet
  }

  let { title, children, actions }: Props = $props()
</script>

<section class="card">
  {#if title || actions}
    <header>
      {#if title}<h2>{title}</h2>{/if}
      {@render actions?.()}
    </header>
  {/if}
  {@render children()}
</section>

<style>
  .card {
    padding: 14px 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  h2 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text-2);
  }
</style>
