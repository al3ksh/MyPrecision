import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { api } from './api'
import type { AppState, Telemetry } from './types'

/** Live data for the open window: latest telemetry (1 s) and the app state. */
class LiveStore {
  telemetry = $state<Telemetry | null>(null)
  app = $state<AppState | null>(null)
  #unlisten: UnlistenFn[] = []

  async start() {
    this.#unlisten = await Promise.all([
      listen<Telemetry>('telemetry', (e) => (this.telemetry = e.payload)),
      listen<AppState>('state-changed', (e) => (this.app = e.payload)),
    ])
    this.app = await api.getState()
  }

  stop() {
    this.#unlisten.forEach((u) => u())
    this.#unlisten = []
  }
}

export const live = new LiveStore()
