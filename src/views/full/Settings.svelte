<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app'
  import { relaunch } from '@tauri-apps/plugin-process'
  import { check, type Update } from '@tauri-apps/plugin-updater'
  import { onMount } from 'svelte'
  import Icon from '../../components/Icon.svelte'
  import { api } from '../../lib/api'
  import { live } from '../../lib/telemetry.svelte'
  import { errorText, toasts } from '../../lib/toasts.svelte'
  import type { DeviceInfo } from '../../lib/types'

  let { device }: { device: DeviceInfo | null } = $props()

  let version = $state<string | null>(null)
  let autostartBusy = $state(false)
  const app = $derived(live.app)

  onMount(() => {
    getVersion().then(
      (v) => (version = v),
      () => {},
    )
  })

  async function toggleAutostart() {
    if (!app) return
    autostartBusy = true
    try {
      const enabled = await api.setAutostart(!app.autostart)
      if (live.app) live.app = { ...live.app, autostart: enabled }
    } catch (e) {
      toasts.push(errorText(e))
    } finally {
      autostartBusy = false
    }
  }

  type UpdateStatus =
    | { kind: 'idle' }
    | { kind: 'checking' }
    | { kind: 'current' }
    | { kind: 'available'; update: Update }
    | { kind: 'installing' }
    | { kind: 'failed'; message: string }
  let update = $state<UpdateStatus>({ kind: 'idle' })

  async function checkForUpdates() {
    update = { kind: 'checking' }
    try {
      const found = await check()
      update = found ? { kind: 'available', update: found } : { kind: 'current' }
    } catch (e) {
      update = { kind: 'failed', message: `Couldn't check for updates: ${errorText(e)}` }
    }
  }

  async function install(found: Update) {
    update = { kind: 'installing' }
    try {
      await found.downloadAndInstall()
      await relaunch()
    } catch (e) {
      update = { kind: 'failed', message: `Couldn't install the update: ${errorText(e)}` }
    }
  }

  const updateText = $derived.by(() => {
    switch (update.kind) {
      case 'checking':
        return 'Checking…'
      case 'current':
        return "You're up to date"
      case 'available':
        return `Version ${update.update.version} is available`
      case 'installing':
        return 'Downloading and installing…'
      case 'failed':
        return update.message
      default:
        return 'Updates come from GitHub Releases and are signature-checked'
    }
  })

  const about = $derived([
    ['Version', version],
    ['Model', device?.model],
    ['Service tag', device?.serviceTag],
    ['BIOS', device?.biosVersion],
    ['BIOS date', device?.biosDate],
  ] as const)
</script>

<div class="stack">
  <section>
    <h2>General</h2>
    <div class="row">
      <Icon name="openWindow" size={20} />
      <div class="text">
        <span>Start at sign-in</span>
        <span class="secondary">Keep the tray icon ready after you sign in to Windows</span>
      </div>
      <button
        type="button"
        role="switch"
        class="switch"
        aria-checked={app?.autostart ?? false}
        aria-label="Start at sign-in"
        disabled={!app || autostartBusy}
        onclick={toggleAutostart}
      >
        <span class="state">{app?.autostart ? 'On' : 'Off'}</span>
        <span class="track"><span class="thumb"></span></span>
      </button>
    </div>
  </section>

  <section>
    <h2>Updates</h2>
    <div class="row">
      <Icon name="download" size={20} />
      <div class="text">
        <span>MyPrecision updates</span>
        <span class="secondary" class:error={update.kind === 'failed'} aria-live="polite">{updateText}</span>
      </div>
      {#if update.kind === 'available'}
        {@const found = update.update}
        <button type="button" class="btn accent" onclick={() => install(found)}>Install and restart</button>
      {:else}
        <button
          type="button"
          class="btn"
          disabled={update.kind === 'checking' || update.kind === 'installing'}
          onclick={checkForUpdates}>Check for updates</button
        >
      {/if}
    </div>
  </section>

  <section>
    <h2>About</h2>
    <div class="row about">
      <Icon name="gauge" size={20} />
      <div class="text">
        <span>MyPrecision</span>
        <span class="secondary">Battery and thermal control for Dell Precision</span>
      </div>
    </div>
    <dl class="row-group">
      {#each about as [k, v] (k)}
        <div><dt class="secondary">{k}</dt><dd class="num">{v ?? '—'}</dd></div>
      {/each}
    </dl>
  </section>
</div>

<style>
  .stack {
    display: grid;
    gap: 24px;
    max-width: 880px;
  }

  h2 {
    margin: 0 0 8px;
    font-size: 14px;
    font-weight: 600;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 68px;
    padding: 12px 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
  }

  .row.about {
    border-bottom-left-radius: 0;
    border-bottom-right-radius: 0;
  }

  .text {
    display: grid;
    gap: 2px;
  }

  .text .secondary {
    font-size: 12px;
  }

  .row-group {
    display: grid;
    margin: 0;
    padding: 4px 16px 8px 52px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-top: none;
    border-radius: 0 0 var(--r-card) var(--r-card);
  }

  .row-group div {
    display: flex;
    justify-content: space-between;
    padding: 6px 0;
    font-size: 13px;
  }

  dd {
    margin: 0;
    user-select: text;
  }

  .error {
    color: #ff99a4;
  }

  .btn {
    flex: none;
    margin-left: auto;
    height: 32px;
    padding: 0 14px;
    background: var(--surface-hover);
    border: 1px solid var(--border);
    border-radius: var(--r-ctl);
    cursor: default;
  }

  .btn:hover:not(:disabled) {
    background: var(--surface);
  }

  .btn:disabled {
    opacity: 0.5;
  }

  .btn.accent {
    background: var(--accent);
    border-color: transparent;
    color: var(--on-accent);
  }

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
