<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte'
  import Banner from '../components/Banner.svelte'
  import Toasts from '../components/Toasts.svelte'
  import { api } from '../lib/api'
  import { bannersFor } from '../lib/banners'
  import { history } from '../lib/history.svelte'
  import { live } from '../lib/telemetry.svelte'
  import { errorText, toasts } from '../lib/toasts.svelte'
  import Battery from './full/Battery.svelte'
  import Overview from './full/Overview.svelte'
  import Sensors from './full/Sensors.svelte'

  const SECTIONS = ['Overview', 'Battery', 'Sensors'] as const
  type Section = (typeof SECTIONS)[number]

  let section = $state<Section>('Overview')
  let autostartBusy = $state(false)

  onMount(() => {
    live.start().catch((e) => toasts.push(errorText(e)))
    history.load().catch((e) => toasts.push(errorText(e)))
  })
  onDestroy(() => {
    live.stop()
    history.clear()
  })

  $effect(() => {
    const t = live.telemetry
    // Only a new tick re-runs this; append reads the samples it replaces.
    if (t) untrack(() => history.append(t))
  })

  const app = $derived(live.app)
  const banners = $derived(app ? bannersFor(app) : [])

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
</script>

<div class="window">
  <nav>
    <div class="brand">MyPrecision</div>
    {#each SECTIONS as s (s)}
      <button type="button" class:active={section === s} aria-current={section === s ? 'page' : undefined} onclick={() => (section = s)}>
        {s}
      </button>
    {/each}
    <footer>
      <button
        type="button"
        role="switch"
        class="switch"
        aria-checked={app?.autostart ?? false}
        aria-label="Start at sign-in"
        disabled={!app || autostartBusy}
        onclick={toggleAutostart}
      >
        <span class="track"><span class="thumb"></span></span>
        <span>Start at sign-in</span>
      </button>
    </footer>
  </nav>

  <main>
    <h1>{section}</h1>
    {#if banners.length}
      <div class="banners">
        {#each banners as id (id)}
          <Banner {id} />
        {/each}
      </div>
    {/if}
    {#if section === 'Overview'}
      <Overview />
    {:else if section === 'Battery'}
      <Battery samples={history.samples} />
    {:else}
      <Sensors samples={history.samples} />
    {/if}

    <Toasts floating />
  </main>
</div>

<style>
  .window {
    height: 100%;
    display: grid;
    grid-template-columns: 200px 1fr;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 16px 8px;
  }

  .brand {
    padding: 4px 12px 14px;
    font-weight: 600;
  }

  nav > button {
    position: relative;
    padding: 8px 12px;
    text-align: left;
    background: none;
    border: none;
    border-radius: var(--r-ctl);
    cursor: default;
  }

  nav > button:hover {
    background: color-mix(in srgb, var(--surface) 70%, transparent);
  }

  nav > button.active {
    background: var(--surface);
  }

  nav > button.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
  }

  footer {
    margin-top: auto;
    padding: 0 4px;
  }

  .switch {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
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

  main {
    min-width: 0;
    overflow-y: auto;
    padding: 24px 28px;
    background: color-mix(in srgb, var(--bg) 70%, transparent);
    border-top-left-radius: var(--r-card);
  }

  h1 {
    margin: 0 0 18px;
    font-size: 24px;
    font-weight: 600;
  }

  .banners {
    display: grid;
    gap: 8px;
    max-width: 720px;
    margin-bottom: 12px;
  }
</style>
