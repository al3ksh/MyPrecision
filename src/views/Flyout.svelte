<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import AnimatedNumber from '../components/AnimatedNumber.svelte'
  import Banner from '../components/Banner.svelte'
  import ModeControls from '../components/ModeControls.svelte'
  import Toasts from '../components/Toasts.svelte'
  import { api } from '../lib/api'
  import { bannersFor } from '../lib/banners'
  import { num, pct, rpm, temp, watts } from '../lib/format'
  import { activeProfileLabel } from '../lib/labels'
  import { live } from '../lib/telemetry.svelte'
  import { errorText, toasts } from '../lib/toasts.svelte'
  import type { BatterySnapshot } from '../lib/types'

  onMount(() => {
    live.start().catch((e) => toasts.push(errorText(e)))
  })
  onDestroy(() => live.stop())

  const app = $derived(live.app)
  // The flyout has room for one banner: the most severe.
  const banner = $derived(app ? bannersFor(app)[0] : undefined)
  const t = $derived(live.telemetry)
  // Telemetry is fresher (1 s); the app state covers the first paint.
  const battery = $derived<BatterySnapshot | null>(t?.battery ?? app?.battery ?? null)

  function powerLine(b: BatterySnapshot | null): string {
    if (!b) return '—'
    if (b.state === 'charging') return `Charging ${watts(b.powerW)}`
    if (b.state === 'discharging') return `Discharging ${watts(b.powerW)}`
    return 'Plugged in — battery bypassed'
  }

  const gpu = $derived.by(() => {
    const g = t?.gpu
    if (!g) return { main: '—', sub: '' }
    if (g.state === 'asleep') return { main: 'Asleep', sub: 'Saving power' }
    if (g.state === 'unavailable') return { main: 'Unavailable', sub: '' }
    return { main: temp(g.tempC), sub: pct(g.loadPct) }
  })

  const mem = $derived(t?.mem ? `${num(t.mem.usedMb / 1024, 1)} / ${num(t.mem.totalMb / 1024, 1)} GB` : '—')
  const memPct = $derived(t?.mem && t.mem.totalMb > 0 ? pct((t.mem.usedMb / t.mem.totalMb) * 100) : '')
</script>

<div class="flyout">
  <header>
    <div class="big num">
      {#if battery?.percent != null}
        <AnimatedNumber value={battery.percent} suffix="%" />
      {:else}
        —
      {/if}
    </div>
    <div class="meta">
      <span class="secondary">{powerLine(battery)}</span>
      <span class="profile">{activeProfileLabel(app?.activeProfile ?? null)}</span>
    </div>
  </header>

  {#if banner}
    <Banner id={banner} />
  {/if}

  <ModeControls disabled={app ? !app.availability.cctk : false} />

  <div class="tiles">
    <div class="tile">
      <span class="name">CPU</span>
      <span class="value num">{temp(t?.cpu.tempC)}</span>
      <span class="sub num">{pct(t?.cpu.loadPct)}</span>
    </div>
    <div class="tile">
      <span class="name">GPU</span>
      <span class="value num">{gpu.main}</span>
      <span class="sub num">{gpu.sub}</span>
    </div>
    <div class="tile">
      <span class="name">Fans</span>
      {#if t && t.fans.length}
        {#each t.fans as f (f.name)}
          <span class="fan num"><span class="secondary">{f.name}</span> {rpm(f.rpm)}</span>
        {/each}
      {:else}
        <span class="value">—</span>
      {/if}
    </div>
    <div class="tile">
      <span class="name">Memory</span>
      <span class="value num">{mem}</span>
      <span class="sub num">{memPct}</span>
    </div>
  </div>

  <Toasts />

  <footer>
    <button type="button" class="open" onclick={() => api.openFullWindow()}>Open full window</button>
  </footer>
</div>

<style>
  .flyout {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px;
    background: color-mix(in srgb, var(--bg) 82%, transparent);
    border: 1px solid var(--border);
    border-radius: var(--r-flyout);
    overflow: hidden;
    animation: enter 150ms var(--ease) both;
  }

  @keyframes enter {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
    to {
      transform: none;
      opacity: 1;
    }
  }

  header {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .big {
    font-size: 40px;
    font-weight: 600;
    line-height: 1;
    min-width: 3.2ch;
  }

  .meta {
    display: grid;
    gap: 2px;
  }

  .profile {
    font-weight: 600;
  }

  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .tile {
    display: grid;
    align-content: start;
    gap: 2px;
    min-height: 72px;
    padding: 10px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
  }

  .name {
    font-size: 12px;
    color: var(--text-2);
  }

  .value {
    font-size: 18px;
    font-weight: 600;
  }

  .sub,
  .fan {
    font-size: 12px;
  }

  footer {
    margin-top: auto;
  }

  .open {
    width: 100%;
    padding: 8px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-ctl);
    cursor: default;
  }

  .open:hover {
    background: #333;
  }
</style>
