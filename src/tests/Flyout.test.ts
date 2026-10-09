import { fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import type { AppState, Telemetry } from '../lib/types'

const invoke = vi.fn()
const handlers: Record<string, (e: { payload: unknown }) => void> = {}

vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, h: (e: { payload: unknown }) => void) => {
    handlers[name] = h
    return () => delete handlers[name]
  },
}))

const { default: Flyout } = await import('../views/Flyout.svelte')
const { live } = await import('../lib/telemetry.svelte')

function appState(over: Partial<AppState> = {}): AppState {
  return {
    charge: { kind: 'Custom', start: 75, stop: 80 },
    activeProfile: { kind: 'known', profile: 'home' },
    thermal: 'Optimized',
    profiles: {
      home: { kind: 'Custom', start: 75, stop: 80 },
      campus: { kind: 'Standard' },
      storage: { kind: 'Custom', start: 50, stop: 60 },
    },
    battery: {
      percent: 82,
      state: 'discharging',
      powerW: -12.5,
      remainingMwh: 70000,
      fullMwh: 85000,
      designMwh: 97000,
      wearPct: 12.4,
      voltageV: 12.6,
      cycles: null,
    },
    availability: { cctk: true, wmi: true, dcm: true, admin: true, optimizerRunning: false },
    autostart: false,
    optimizerWarningDismissed: false,
    ...over,
  }
}

function telemetry(over: Partial<Telemetry> = {}): Telemetry {
  return {
    tsMs: 0,
    battery: null,
    cpu: { loadPct: 7, tempC: 54 },
    gpu: { state: 'active', tempC: 60, loadPct: 3 },
    fans: [
      { name: 'CPU', rpm: 2100 },
      { name: 'GPU', rpm: 1900 },
    ],
    mem: { usedMb: 10240, totalMb: 32768 },
    dimmC: null,
    skinC: null,
    ...over,
  }
}

/** Controls stay disabled until the first state arrives. */
async function loaded() {
  await waitFor(() => expect(screen.getByRole('radio', { name: 'Home' }).getAttribute('aria-checked')).toBe('true'))
}

function deferred<T>() {
  let resolve!: (v: T) => void
  let reject!: (e: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

beforeEach(() => {
  invoke.mockReset()
})

afterEach(() => live.stop())

describe('Flyout', () => {
  test('Other profile shows "Custom (60–90%)" and no segment checked', async () => {
    const s = appState({ activeProfile: { kind: 'other', cfg: { kind: 'Custom', start: 60, stop: 90 } } })
    invoke.mockResolvedValue(s)
    render(Flyout)
    expect(await screen.findByText('Custom (60–90%)')).toBeTruthy()
    const group = screen.getByRole('radiogroup', { name: 'Battery profile' })
    const radios = group.querySelectorAll('[role="radio"]')
    expect([...radios].every((r) => r.getAttribute('aria-checked') === 'false')).toBe(true)
  })

  test('clicking Storage calls setBatteryProfile("storage") and disables control until resolved', async () => {
    const pending = deferred<AppState>()
    invoke.mockImplementation((cmd: string) =>
      cmd === 'get_state' ? Promise.resolve(appState()) : pending.promise,
    )
    render(Flyout)
    await loaded()
    await fireEvent.click(screen.getByRole('radio', { name: 'Storage' }))
    expect(invoke).toHaveBeenCalledWith('set_battery_profile', { profile: 'storage' })
    await fireEvent.click(screen.getByRole('radio', { name: 'Campus' }))
    expect(invoke.mock.calls.filter(([c]) => c === 'set_battery_profile')).toHaveLength(1)
    pending.resolve(appState({ activeProfile: { kind: 'known', profile: 'storage' } }))
    await waitFor(() => expect(screen.getByRole('radio', { name: 'Storage' }).getAttribute('aria-checked')).toBe('true'))
  })

  test('reports ready only after the first state has rendered, so the window never shows blank', async () => {
    const state = deferred<AppState>()
    invoke.mockImplementation((cmd: string) => (cmd === 'get_state' ? state.promise : Promise.resolve()))
    render(Flyout)
    await Promise.resolve()
    expect(invoke).not.toHaveBeenCalledWith('window_ready')
    state.resolve(appState())
    await loaded()
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('window_ready'))
  })

  test('clicked thermal mode is shown selected at once while the BIOS write runs', async () => {
    const pending = deferred<AppState>()
    invoke.mockImplementation((cmd: string) =>
      cmd === 'get_state' ? Promise.resolve(appState()) : pending.promise,
    )
    render(Flyout)
    await loaded()
    await fireEvent.click(screen.getByRole('radio', { name: 'Cool' }))
    expect(screen.getByRole('radio', { name: 'Cool' }).getAttribute('aria-checked')).toBe('true')
    expect(screen.getByRole('radiogroup', { name: 'Thermal mode' }).getAttribute('aria-busy')).toBe('true')
    pending.resolve(appState({ thermal: 'Cool' }))
    await waitFor(() =>
      expect(screen.getByRole('radiogroup', { name: 'Thermal mode' }).getAttribute('aria-busy')).toBe('false'),
    )
    expect(screen.getByRole('radio', { name: 'Cool' }).getAttribute('aria-checked')).toBe('true')
  })

  test('failed thermal write reverts the selection to the BIOS value', async () => {
    const pending = deferred<AppState>()
    invoke.mockImplementation((cmd: string) =>
      cmd === 'get_state' ? Promise.resolve(appState()) : pending.promise,
    )
    render(Flyout)
    await loaded()
    await fireEvent.click(screen.getByRole('radio', { name: 'Quiet' }))
    pending.reject('cctk exited with code 41')
    await waitFor(() =>
      expect(screen.getByRole('radio', { name: 'Optimized' }).getAttribute('aria-checked')).toBe('true'),
    )
  })

  test('rejected setThermalMode shows error toast text', async () => {
    invoke.mockImplementation((cmd: string) =>
      cmd === 'get_state' ? Promise.resolve(appState()) : Promise.reject('cctk exited with code 58'),
    )
    render(Flyout)
    await loaded()
    await fireEvent.click(screen.getByRole('radio', { name: 'Cool' }))
    expect(await screen.findByText('cctk exited with code 58')).toBeTruthy()
  })

  test('gpu Asleep renders "Asleep"', async () => {
    invoke.mockResolvedValue(appState())
    render(Flyout)
    await screen.findByRole('radio', { name: 'Home' })
    handlers['telemetry']({ payload: telemetry({ gpu: { state: 'asleep' } }) })
    expect(await screen.findByText('Asleep')).toBeTruthy()
  })

  test('header shows discharging power from state before telemetry', async () => {
    invoke.mockResolvedValue(appState())
    render(Flyout)
    expect(await screen.findByText('12.5 W')).toBeTruthy()
    expect(screen.getByText('On battery')).toBeTruthy()
  })

  test('footer icon button opens the full window', async () => {
    invoke.mockResolvedValue(appState())
    render(Flyout)
    await loaded()
    await fireEvent.click(screen.getByRole('button', { name: 'Open full window' }))
    expect(invoke).toHaveBeenCalledWith('open_full_window')
  })

  test('profile tiles show each profile range', async () => {
    invoke.mockResolvedValue(appState())
    render(Flyout)
    await loaded()
    expect(screen.getByRole('radio', { name: 'Home' }).textContent).toContain('75–80%')
    expect(screen.getByRole('radio', { name: 'Campus' }).textContent).toContain('Up to 100%')
  })

  test('sensor list shows CPU temperature and load, and both fans', async () => {
    invoke.mockResolvedValue(appState())
    render(Flyout)
    await loaded()
    handlers['telemetry']({ payload: telemetry() })
    expect(await screen.findByText('54 °C')).toBeTruthy()
    expect(screen.getByText('2,100 · 1,900')).toBeTruthy()
  })

  test('window height follows the content as it changes', async () => {
    let observed: ResizeObserverCallback | undefined
    vi.stubGlobal(
      'ResizeObserver',
      class {
        constructor(cb: ResizeObserverCallback) {
          observed = cb
        }
        observe() {}
        disconnect() {}
      },
    )
    invoke.mockResolvedValue(appState())
    render(Flyout)
    await loaded()
    const entry = { borderBoxSize: [{ blockSize: 431.4, inlineSize: 360 }] } as unknown as ResizeObserverEntry
    observed?.([entry], {} as ResizeObserver)
    expect(invoke).toHaveBeenCalledWith('fit_flyout', { height: 432 })
    vi.unstubAllGlobals()
  })
})
