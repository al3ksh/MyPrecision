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
      cycles: null,
    },
    availability: { cctk: true, dcm: true, admin: true, optimizerRunning: false },
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
    expect(await screen.findByText('Discharging 12.5 W')).toBeTruthy()
  })
})
