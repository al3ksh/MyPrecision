<script lang="ts">
  import Card from '../../components/Card.svelte'
  import LineChart from '../../components/LineChart.svelte'
  import { num, pct, rpm, temp } from '../../lib/format'
  import { HISTORY_MINUTES } from '../../lib/history.svelte'
  import { live } from '../../lib/telemetry.svelte'
  import type { HistorySample } from '../../lib/types'

  let { samples }: { samples: HistorySample[] } = $props()

  const t = $derived(live.telemetry)
  const series = (key: 'cpuTemp' | 'cpuLoad' | 'gpuTemp') => samples.map((s) => ({ t: s.tsMs, v: s[key] }))
  const cpuTemp = $derived(series('cpuTemp'))
  const cpuLoad = $derived(series('cpuLoad'))
  const gpuTemp = $derived(series('gpuTemp'))

  const gpuNow = $derived.by(() => {
    const g = t?.gpu
    if (!g) return '—'
    if (g.state === 'asleep') return 'Asleep'
    if (g.state === 'unavailable') return 'Unavailable'
    return temp(g.tempC)
  })
</script>

<div class="stack">
  <div class="tiles">
    <div class="tile"><span class="name">Memory module</span><span class="value num">{temp(t?.dimmC)}</span></div>
    <div class="tile"><span class="name">Chassis</span><span class="value num">{temp(t?.skinC)}</span></div>
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
      <span class="name">RAM</span>
      <span class="value num">
        {t?.mem ? `${num(t.mem.usedMb / 1024, 1)} / ${num(t.mem.totalMb / 1024, 1)} GB` : '—'}
      </span>
    </div>
  </div>

  <Card title="CPU temperature — last {HISTORY_MINUTES} minutes">
    {#snippet actions()}<span class="now num">{temp(t?.cpu.tempC)}</span>{/snippet}
    <LineChart samples={cpuTemp} minutes={HISTORY_MINUTES} unit="°" />
  </Card>

  <Card title="CPU load — last {HISTORY_MINUTES} minutes">
    {#snippet actions()}<span class="now num">{pct(t?.cpu.loadPct)}</span>{/snippet}
    <LineChart samples={cpuLoad} minutes={HISTORY_MINUTES} unit="%" min={0} max={100} />
  </Card>

  <Card title="GPU temperature — last {HISTORY_MINUTES} minutes">
    {#snippet actions()}<span class="now num">{gpuNow}</span>{/snippet}
    <LineChart samples={gpuTemp} minutes={HISTORY_MINUTES} unit="°" />
  </Card>
</div>

<style>
  .stack {
    display: grid;
    gap: 12px;
    max-width: 720px;
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }

  .tile {
    display: grid;
    align-content: start;
    gap: 2px;
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
    font-weight: 500;
  }

  .fan {
    font-size: 12px;
  }

  .now {
    font-weight: 500;
  }
</style>
