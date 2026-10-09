<script lang="ts">
  import type { IconName } from '../lib/icons'
  import Icon from './Icon.svelte'

  interface Option {
    value: string
    /** Accessible name; also shown unless `short` is given. */
    label: string
    short?: string
    icon?: IconName
    sub?: string
  }

  interface Props {
    options: Option[]
    /** `null` = the current setting matches no option (e.g. a custom BIOS range). */
    value: string | null
    disabled?: boolean
    /** A change is in flight: block input, show progress. */
    busy?: boolean
    label?: string
    onchange: (value: string) => void
  }

  let { options, value, disabled = false, busy = false, label, onchange }: Props = $props()

  const index = $derived(options.findIndex((o) => o.value === value))
  const locked = $derived(disabled || busy)

  function select(v: string) {
    if (locked || v === value) return
    onchange(v)
  }

  // Manual activation: every selection is a BIOS write, so arrows only move focus and
  // Enter/Space (the button's own click) commits.
  function onkeydown(e: KeyboardEvent, i: number) {
    const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0
    if (!step) return
    e.preventDefault()
    const next = (i + step + options.length) % options.length
    const group = (e.currentTarget as HTMLElement).parentElement
    group?.querySelectorAll<HTMLElement>('[role="radio"]')[next]?.focus()
  }
</script>

<div
  class="tiles"
  class:locked
  class:busy
  role="radiogroup"
  aria-label={label}
  aria-disabled={locked}
  aria-busy={busy}
  style:--n={options.length}
>
  {#each options as o, i (o.value)}
    <button
      type="button"
      role="radio"
      aria-label={o.label}
      aria-checked={i === index}
      tabindex={i === index || (index < 0 && i === 0) ? 0 : -1}
      class:checked={i === index}
      class:centered={!o.sub}
      onclick={() => select(o.value)}
      onkeydown={(e) => onkeydown(e, i)}
    >
      {#if o.icon}<Icon name={o.icon} />{/if}
      <span class="name">{o.short ?? o.label}</span>
      {#if o.sub}<span class="sub num">{o.sub}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(var(--n), 1fr);
    gap: 8px;
  }

  button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    min-width: 0;
    padding: 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition:
      background 150ms var(--ease),
      color 150ms var(--ease);
  }

  button.centered {
    align-items: center;
    text-align: center;
  }

  button:hover:not(.checked) {
    background: var(--surface-hover);
  }

  button:active:not(.checked) {
    background: var(--surface);
    color: var(--text-2);
  }

  button.checked {
    background: var(--accent);
    border-color: transparent;
    color: var(--on-accent);
  }

  .name {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .sub {
    font-size: 11px;
    opacity: 0.75;
  }

  .locked button {
    cursor: default;
  }

  .tiles[aria-disabled='true']:not(.busy) {
    opacity: 0.5;
  }

  .busy button.checked {
    animation: pulse 900ms ease-in-out infinite alternate;
  }

  @keyframes pulse {
    to {
      opacity: 0.6;
    }
  }
</style>
