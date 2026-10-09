<script lang="ts">
  interface Option {
    value: string
    label: string
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

  function onkeydown(e: KeyboardEvent, i: number) {
    const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0
    if (!step) return
    e.preventDefault()
    const next = (i + step + options.length) % options.length
    select(options[next].value)
    const group = (e.currentTarget as HTMLElement).parentElement
    group?.querySelectorAll<HTMLElement>('[role="radio"]')[next]?.focus()
  }
</script>

<div
  class="seg"
  class:locked
  class:busy
  role="radiogroup"
  aria-label={label}
  aria-disabled={locked}
  aria-busy={busy}
  style:--n={options.length}
>
  {#if index >= 0}
    <span class="indicator" style:transform="translateX({index * 100}%)"></span>
  {/if}
  {#each options as o, i (o.value)}
    <button
      type="button"
      role="radio"
      aria-checked={i === index}
      tabindex={i === index || (index < 0 && i === 0) ? 0 : -1}
      class:checked={i === index}
      onclick={() => select(o.value)}
      onkeydown={(e) => onkeydown(e, i)}
    >
      {o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--n), 1fr);
    padding: 3px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--r-ctl);
  }

  .indicator {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    width: calc((100% - 6px) / var(--n));
    background: var(--accent);
    border-radius: calc(var(--r-ctl) - 3px);
    transition: transform 180ms var(--ease);
  }

  .busy .indicator {
    animation: pulse 900ms ease-in-out infinite alternate;
  }

  button {
    position: relative;
    padding: 6px 8px;
    background: none;
    border: 0;
    border-radius: calc(var(--r-ctl) - 3px);
    color: var(--text-2);
    cursor: pointer;
    white-space: nowrap;
    transition: color 180ms var(--ease);
  }

  button:hover:not(.checked) {
    color: var(--text);
  }

  button.checked {
    color: var(--on-accent);
    font-weight: 600;
  }

  .locked button {
    cursor: default;
  }

  .seg[aria-disabled='true']:not(.busy) {
    opacity: 0.5;
  }

  @keyframes pulse {
    to {
      opacity: 0.6;
    }
  }
</style>
