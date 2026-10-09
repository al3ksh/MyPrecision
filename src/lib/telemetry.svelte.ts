import { tick } from 'svelte'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { api } from './api'
import type { AppState, Telemetry } from './types'

/** Live data for the open window: latest telemetry (1 s) and the app state. */
class LiveStore {
  telemetry = $state<Telemetry | null>(null)
  app = $state<AppState | null>(null)
  #unlisten: UnlistenFn[] = []

  /** First paint comes from `getState`, without waiting for the first telemetry tick. */
  async start() {
    const [unlisten, app] = await Promise.all([
      Promise.all([
        listen<Telemetry>('telemetry', (e) => (this.telemetry = e.payload)),
        listen<AppState>('state-changed', (e) => (this.app = e.payload)),
      ]),
      api.getState(),
    ])
    this.#unlisten = unlisten
    // A state-changed event that raced getState is newer; keep it.
    this.app ??= app
  }

  stop() {
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
