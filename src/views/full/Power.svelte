<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import Icon from '../../components/Icon.svelte'
  import { api } from '../../lib/api'
  import { num } from '../../lib/format'
  import { errorText, toasts } from '../../lib/toasts.svelte'
  import type { DgpuReport, GpuApp } from '../../lib/types'

  /** The card wakes and sleeps as apps come and go. */
  const REFRESH_MS = 5000

  let dgpu = $state<DgpuReport | null | undefined>(undefined)
  let error = $state<string | null>(null)
  let busy = $state<string | null>(null)
  let timer: ReturnType<typeof setInterval> | undefined

  async function load() {
    try {
      dgpu = await api.getDgpu()
      error = null
    } catch (e) {
      if (dgpu === undefined) error = errorText(e)
    }
  }

  onMount(() => {
    load()
    timer = setInterval(load, REFRESH_MS)
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
</script>

<div class="stack">
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
</div>

<style>
  .stack {
    display: grid;
    gap: 20px;
    max-width: 880px;
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
