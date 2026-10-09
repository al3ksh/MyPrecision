import { tick } from 'svelte'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { api } from './api'
import type { AppState, Telemetry } from './types'

/** How long the first paint waits for a telemetry tick when the poller has none cached. */
export const FIRST_TICK_WAIT_MS = 600

/** Live data for the open window: latest telemetry (1 s) and the app state. */
class LiveStore {
  telemetry = $state<Telemetry | null>(null)
  app = $state<AppState | null>(null)
  #unlisten: UnlistenFn[] = []
  #firstTick: (() => void) | null = null

  /** First paint has both the app state and a telemetry tick, so values don't pop in after the window shows. */
  async start() {
    const [unlisten, , cached] = await Promise.all([
      Promise.all([
        listen<Telemetry>('telemetry', (e) => {
          this.telemetry = e.payload
          this.#firstTick?.()
        }),
        listen<AppState>('state-changed', (e) => (this.app = e.payload)),
      ]),
      // Controls render as soon as the state is in, whatever telemetry is doing.
      // A state-changed event that raced this request is newer; keep it.
      api.getState().then((app) => (this.app ??= app)),
      api.getTelemetry().catch(() => null),
    ])
    this.#unlisten = unlisten
    this.telemetry ??= cached ?? null
    if (!this.telemetry) await this.#waitForTick()
  }

  #waitForTick() {
    return new Promise<void>((resolve) => {
      const done = () => {
        clearTimeout(timer)
        this.#firstTick = null
        resolve()
      }
      const timer = setTimeout(done, FIRST_TICK_WAIT_MS)
      this.#firstTick = done
    })
  }

  stop() {
    this.#firstTick?.()
    this.#unlisten.forEach((u) => u())
    this.#unlisten = []
    this.telemetry = null
    this.app = null
  }
}

export const live = new LiveStore()

/**
 * Starts the live store and reveals the window once its first state has painted
 * (or failed to load, so the error toast is visible).
 */
export async function startWindow(onError: (e: unknown) => void, beforeReveal?: () => void | Promise<void>) {
  try {
    await live.start()
  } catch (e) {
    onError(e)
  }
  await tick()
  await beforeReveal?.()
  await api.windowReady().catch(() => {})
}
