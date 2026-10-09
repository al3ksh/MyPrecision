<script lang="ts">
  import AnimatedNumber from '../../components/AnimatedNumber.svelte'
  import Card from '../../components/Card.svelte'
  import Icon from '../../components/Icon.svelte'
  import ModeControls from '../../components/ModeControls.svelte'
  import Sparkline from '../../components/Sparkline.svelte'
  import { num, pct, temp, watts, wh } from '../../lib/format'
  import type { IconName } from '../../lib/icons'
  import { activeProfileLabel, PROFILE_LABEL } from '../../lib/labels'
  import { live } from '../../lib/telemetry.svelte'
  import type { HistorySample } from '../../lib/types'

  let { samples }: { samples: HistorySample[] } = $props()

  const app = $derived(live.app)
  const t = $derived(live.telemetry)
  const battery = $derived(t?.battery ?? app?.battery ?? null)

  const status = $derived.by(() => {
    const b = battery
    const power = !b
      ? '—'
      : b.state === 'charging'
        ? 'Charging'
        : b.state === 'discharging'
          ? 'On battery'
          : 'Plugged in · battery bypassed'
    const a = app?.activeProfile
    const profile = !a ? null : a.kind === 'known' ? `${PROFILE_LABEL[a.profile]} profile` : activeProfileLabel(a)
    return profile ? `${power} · ${profile}` : power
  })
  const healthPct = $derived(battery?.wearPct != null ? 100 - battery.wearPct : null)

  interface Live {
    icon: IconName
    name: string
    value: string
    sub: string
    trend?: { t: number; v: number | null }[]
  }

  const cards = $derived.by((): Live[] => {
    const g = t?.gpu
    const mem = t?.mem
    return [
      {
        icon: 'cpu',
        name: 'CPU',
        value: temp(t?.cpu.tempC),
        sub: `${pct(t?.cpu.loadPct)} load`,
        trend: samples.map((s) => ({ t: s.tsMs, v: s.cpuLoad })),
      },
      {
        icon: 'gpu',
        name: 'GPU',
        value: !g || g.state === 'unavailable' ? '—' : g.state === 'asleep' ? 'Asleep' : temp(g.tempC),
        sub: g?.state === 'active' ? `${pct(g.loadPct)} load` : 'Saving power',
      },
      {
        icon: 'fan',
        name: 'Fans',
        value: t?.fans.length ? t.fans.map((f) => num(f.rpm)).join(' · ') : '—',
        sub: 'RPM',
      },
      {
        icon: 'memory',
        name: 'Memory',
        value: mem && mem.totalMb > 0 ? pct((mem.usedMb / mem.totalMb) * 100) : '—',
        sub: mem ? `${num(mem.usedMb / 1024, 1)} of ${num(mem.totalMb / 1024, 0)} GB` : '',
      },
    ]
  })
</script>

<div class="page">
  <section class="hero">
    <span class="hero-icon"><Icon name={battery?.state === 'discharging' ? 'battery' : 'batteryCharging'} size={32} /></span>
    <div class="charge">
      <span class="big num">
        {#if battery?.percent != null}<AnimatedNumber value={battery.percent} suffix="%" />{:else}—{/if}
      </span>
      <span class="secondary">{status}</span>
      {#if battery && battery.state !== 'bypass'}
        <span class="secondary num">{watts(battery.powerW)}</span>
      {/if}
    </div>
    <div class="health">
      <span class="secondary">Battery health</span>
      <span class="mid num">{pct(healthPct, 1)}</span>
      <span class="secondary num">{wh(battery?.fullMwh)} of {wh(battery?.designMwh)}</span>
    </div>
  </section>

  <Card>
    <div class="controls">
      <ModeControls describe disabled={app ? !app.availability.cctk : false} />
    </div>
  </Card>

  <section>
    <h2>Right now</h2>
    <div class="live">
      {#each cards as c (c.name)}
        <div class="tile">
          <div class="tile-head"><Icon name={c.icon} /><span>{c.name}</span></div>
          <span class="value num">{c.value}</span>
          <span class="secondary num small">{c.sub}</span>
          {#if c.trend}<Sparkline samples={c.trend} />{/if}
        </div>
      {/each}
    </div>
  </section>
</div>

<style>
  .page {
    display: grid;
    gap: 16px;
    max-width: 880px;
  }

  .hero {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 20px 24px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
  }

  .hero-icon {
    color: var(--accent);
  }

  .charge {
    display: grid;
    gap: 4px;
  }

  .big {
    font-size: 44px;
    font-weight: 300;
    letter-spacing: -0.02em;
    line-height: 1;
  }

  .health {
    display: grid;
    gap: 2px;
    margin-left: auto;
    text-align: right;
    font-size: 12px;
  }

  .mid {
    font-size: 24px;
    font-weight: 400;
  }

  .controls {
    display: grid;
    gap: 16px;
  }

  h2 {
    margin: 0 0 8px;
    font-size: 14px;
    font-weight: 600;
  }

  .live {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }

  .tile {
    display: grid;
    align-content: start;
    gap: 4px;
    min-width: 0;
    padding: 12px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
  }

  .tile-head {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-2);
  }

  .value {
    font-size: 20px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .small {
    font-size: 12px;
  }
</style>
