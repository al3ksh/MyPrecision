<script lang="ts">
  /** Windows 11 toggle switch with its On/Off caption. */
  let {
    checked,
    label,
    disabled = false,
    onchange,
  }: { checked: boolean; label: string; disabled?: boolean; onchange: () => void } = $props()
</script>

<button
  type="button"
  role="switch"
  class="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={onchange}
>
  <span class="state">{checked ? 'On' : 'Off'}</span>
  <span class="track"><span class="thumb"></span></span>
</button>

<style>
  .switch {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-left: auto;
    padding: 4px;
    background: none;
    border: none;
    cursor: default;
  }

  .switch:disabled {
    opacity: 0.5;
  }

  .track {
    position: relative;
    width: 40px;
    height: 20px;
    border: 1px solid var(--text-2);
    border-radius: 10px;
    transition: background 150ms var(--ease);
  }

  .thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-2);
    transition: transform 150ms var(--ease);
  }

  .switch[aria-checked='true'] .track {
    background: var(--accent);
    border-color: var(--accent);
  }

  .switch[aria-checked='true'] .thumb {
    transform: translateX(20px);
    background: var(--on-accent);
  }
</style>
