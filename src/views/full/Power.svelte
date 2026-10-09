<script lang="ts" module>
  import type { AppEnergy, DgpuReport, EnergyReport, GpuApp, SleepReport, SleepSession } from '../../lib/types'

  // Both run powercfg for a few seconds; keep them across section switches.
  let energyCache: EnergyReport | null = null
  let sleepCache: SleepReport | null = null

  /** Drops the cached reads; tests start each case fresh. */
  export function forget() {
    energyCache = null
    sleepCache = null
  }

  /** Average sleep drain at or below this, in % of a full battery per hour, is normal. */
  const NORMAL_DRAIN_PER_HOUR = 1.5
</script>

<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import Icon from '../../components/Icon.svelte'
  import { api } from '../../lib/api'
  import { num, pct, wh } from '../../lib/format'
  import { errorText, toasts } from '../../lib/toasts.svelte'

  /** The card wakes and sleeps as apps come and go. */
  const REFRESH_MS = 5000

  let dgpu = $state<DgpuReport | null | undefined>(undefined)
  let error = $state<string | null>(null)
  let busy = $state<string | null>(null)
  let timer: ReturnType<typeof setInterval> | undefined

  let energy = $state<EnergyReport | null>(energyCache)
  let sleep = $state<SleepReport | null>(sleepCache)
  let energyError = $state<string | null>(null)
  let sleepError = $state<string | null>(null)
  let reading = $state(false)
  let week = $state(false)

  async function load() {
    try {
      dgpu = await api.getDgpu()
      error = null
    } catch (e) {
      if (dgpu === undefined) error = errorText(e)
    }
  }

  async function loadBattery() {
    reading = true
    energyError = sleepError = null
    await Promise.all([
      api.getAppEnergy().then(
        (r) => (energy = energyCache = r),
        (e) => (energy ? toasts.push(errorText(e)) : (energyError = errorText(e))),
      ),
      api.getSleep().then(
        (r) => (sleep = sleepCache = r),
        (e) => (sleep ? toasts.push(errorText(e)) : (sleepError = errorText(e))),
      ),
    ])
    reading = false
  }

  onMount(() => {
    load()
    timer = setInterval(load, REFRESH_MS)
    if (!energy || !sleep) loadBattery()
  })
  onDestroy(() => clearInterval(timer))

  async function setIntegrated(app: GpuApp, on: boolean) {
    busy = app.path
    try {
      await api.setIntegratedGpu(app.path, on)
      await load()
    } catch (e) {
      toasts.push(errorText(e))
    } finally {
      busy = null
    }
  }

  const mb = (b: number) => num(b / 1e6, 0, ' MB')

  function summary(d: DgpuReport): string {
    if (!d.active) return "No app is using it, so it's off and saving battery."
    const n = d.apps.length
    if (n === 0) return "It's on, but no app is holding it. It should turn off shortly."
    return `${n} ${n === 1 ? 'app keeps' : 'apps keep'} it awake. It draws several watts while on; apps that don't need it can run on the integrated GPU instead.`
  }

  const apps = $derived<AppEnergy[]>(energy ? (week ? energy.week : energy.day) : [])
  const total = $derived(energy ? (week ? energy.weekTotalMwh : energy.dayTotalMwh) : 0)
  const top = $derived(Math.max(1, ...apps.map((a) => a.mwh)))

  /** Share of a full battery. */
  const used = (s: SleepSession) => (s.fullMwh ? (s.drainedMwh * 100) / s.fullMwh : 0)
  const perHour = (s: SleepSession) => used(s) / (s.minutes / 60)

  function sleepSummary(sessions: SleepSession[]): string {
    const hours = sessions.reduce((n, s) => n + s.minutes / 60, 0)
    const rate = sessions.reduce((n, s) => n + used(s), 0) / hours
    const verdict =
      rate <= NORMAL_DRAIN_PER_HOUR ? "That's normal." : "That's more than it should; see what kept it awake below."
    return `This laptop loses about ${num(rate, 1, '%')} an hour asleep on battery. ${verdict}`
  }

  function duration(m: number): string {
    if (m < 60) return `${m} min`
    const [h, r] = [Math.floor(m / 60), m % 60]
    return r ? `${h} h ${r} min` : `${h} h`
  }

  const date = (iso: string) =>
    new Date(iso).toLocaleString('en-US', { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
</script>

<div class="stack">
  <div class="note">
    <Icon name="info" />
    <span class="secondary">Battery use per app is Windows' own estimate and leaves out the NVIDIA GPU.</span>
    {#if energy || sleep}
      <button type="button" class="btn" disabled={reading} onclick={loadBattery}>
        <Icon name="refresh" />
        {reading ? 'Reading…' : 'Refresh'}
      </button>
    {/if}
  </div>

  <section>
    <h2>Graphics</h2>
    {#if dgpu}
      <div class="card">
        <div class="head">
          <Icon name="gpu" />
          <strong>{dgpu.name ?? 'NVIDIA GPU'}</strong>
          <span class="state right" class:on={dgpu.active}>{dgpu.active ? 'Active' : 'Sleeping'}</span>
        </div>
        <p class="secondary">{summary(dgpu)}</p>
      </div>

      {#if dgpu.apps.length}
        <div class="card list">
          {#each dgpu.apps as a (a.path)}
            <div class="row">
              <div class="text">
                <span>{a.name}</span>
                <span class="secondary" title={a.path}>{mb(a.bytes)} on the GPU</span>
              </div>
              {#if a.integrated}
                <span class="secondary">Restart {a.name} to switch.</span>
                <button type="button" class="btn" disabled={busy === a.path} onclick={() => setIntegrated(a, false)}>
                  Undo
                </button>
              {:else}
                <button type="button" class="btn" disabled={busy === a.path} onclick={() => setIntegrated(a, true)}>
                  Use integrated graphics
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      {#if dgpu.integrated.length}
        <h3>Set to integrated graphics</h3>
        <div class="card list">
          {#each dgpu.integrated as a (a.path)}
            <div class="row">
              <div class="text">
                <span>{a.name}</span>
                <span class="secondary path">{a.path}</span>
              </div>
              <button type="button" class="btn" disabled={busy === a.path} onclick={() => setIntegrated(a, false)}>
                Undo
              </button>
            </div>
          {/each}
        </div>
      {/if}
    {:else if dgpu === null}
      <div class="card"><p class="secondary">This laptop has integrated graphics only.</p></div>
    {:else if error}
      <div class="card"><p>{error}</p></div>
    {:else}
      <div class="loading" role="status"><span class="spinner"></span></div>
    {/if}
  </section>

  <section>
    <div class="head">
      <h2>Battery use by app</h2>
      {#if energy}
        <div class="toggle right">
          <button type="button" class="btn" aria-pressed={!week} onclick={() => (week = false)}>24 hours</button>
          <button type="button" class="btn" aria-pressed={week} onclick={() => (week = true)}>7 days</button>
        </div>
      {/if}
    </div>
    {#if energy}
      {#if apps.length}
        <div class="card list energy">
          {#each apps as a (a.name)}
            <div class="row">
              <div class="text">
                <span>{a.name}</span>
                {#if a.screenOffMwh}
                  <span class="secondary">{wh(a.screenOffMwh)} with the screen off</span>
                {/if}
              </div>
              <span class="bar" aria-hidden="true"><span style:width="{(a.mwh * 100) / top}%"></span></span>
              <span class="amount">{wh(a.mwh)}</span>
              <span class="share secondary">{pct(total ? (a.mwh * 100) / total : 0)}</span>
            </div>
          {/each}
        </div>
      {:else}
        <div class="card"><p class="secondary">No time on battery in this period.</p></div>
      {/if}
    {:else if energyError}
      <div class="card"><p>{energyError}</p></div>
    {:else}
      <div class="loading" role="status"><span class="spinner"></span></div>
    {/if}
  </section>

  <section>
    <h2>Sleep</h2>
    {#if sleep}
      {#if sleep.sessions.length}
        <div class="card"><p class="secondary">{sleepSummary(sleep.sessions)}</p></div>
        <div class="card list">
          {#each sleep.sessions as s (s.start)}
            <div class="row">
              <div class="text">
                <span>{date(s.start)} · {duration(s.minutes)} asleep</span>
                {#if s.blocker}
                  <span class="secondary warn">Kept awake by {s.blocker}</span>
                {/if}
                <span class="secondary">{s.deepPct}% deep sleep</span>
              </div>
              <span class="amount">{num(used(s), 0, '% used')}</span>
              <span class="share secondary" class:warn={perHour(s) > NORMAL_DRAIN_PER_HOUR}>
                {num(perHour(s), 1, '%/h')}
              </span>
            </div>
          {/each}
        </div>
      {:else}
        <div class="card"><p class="secondary">No sleep on battery recorded in the last week.</p></div>
      {/if}
    {:else if sleepError}
      <div class="card"><p>{sleepError}</p></div>
    {:else}
      <div class="loading" role="status"><span class="spinner"></span></div>
    {/if}
  </section>
</div>

<style>
  .stack {
    display: grid;
    gap: 20px;
    max-width: 880px;
  }

  .note {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }

  .note .btn {
    margin-left: auto;
  }

  section {
    display: grid;
    gap: 8px;
  }

  h2,
  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  h3 {
    margin-top: 8px;
    font-size: 13px;
  }

  .card {
    display: grid;
    gap: 10px;
    padding: 14px 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
  }

  .card p {
    margin: 0;
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .right {
    margin-left: auto;
  }

  .state {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .state.on {
    color: var(--accent);
  }

  .list {
    gap: 0;
    padding: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 52px;
    padding: 8px 16px;
  }

  .row + .row {
    border-top: 1px solid var(--border);
  }

  .row > .secondary {
    font-size: 12px;
  }

  .text {
    display: grid;
    gap: 2px;
    min-width: 0;
    margin-right: auto;
  }

  .text .secondary {
    font-size: 12px;
  }

  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bar {
    display: block;
    flex: none;
    width: 160px;
    height: 6px;
    overflow: hidden;
    background: var(--surface-hover);
    border-radius: 3px;
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
  }

  .amount {
    flex: none;
    min-width: 64px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .share {
    flex: none;
    min-width: 48px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .warn {
    color: var(--danger, #e81123);
  }

  .toggle {
    display: flex;
    gap: 4px;
  }

  .toggle .btn {
    height: 28px;
    padding: 0 10px;
    font-size: 12px;
  }

  .btn {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 32px;
    padding: 0 14px;
    color: inherit;
    font: inherit;
    background: var(--surface-hover);
    border: 1px solid var(--border);
    border-radius: var(--r-ctl);
    cursor: default;
  }

  .btn:hover:not(:disabled) {
    background: var(--surface);
  }

  .btn[aria-pressed='true'] {
    color: var(--on-accent);
    background: var(--accent);
    border-color: transparent;
  }

  .btn:disabled {
    opacity: 0.5;
  }

  .loading {
    display: grid;
    justify-items: center;
    padding: 32px 0;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
