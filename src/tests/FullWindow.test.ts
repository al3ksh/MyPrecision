import { fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import type { AppState } from '../lib/types'

const invoke = vi.fn()
const handlers: Record<string, (e: { payload: unknown }) => void> = {}

vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }))
vi.mock('@tauri-apps/api/app', () => ({ getVersion: () => Promise.resolve('0.1.0') }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, h: (e: { payload: unknown }) => void) => {
    handlers[name] = h
    return () => delete handlers[name]
  },
}))

const { default: FullWindow } = await import('../views/FullWindow.svelte')
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

function route(state: AppState) {
  invoke.mockImplementation((cmd: string, args?: { enabled?: boolean }) => {
    if (cmd === 'get_state') return Promise.resolve(state)
    if (cmd === 'get_history' || cmd === 'get_health_log') return Promise.resolve([])
    if (cmd === 'set_autostart') return Promise.resolve(args?.enabled ?? false)
    if (cmd === 'window_ready') return Promise.resolve()
    if (cmd === 'get_device_info')
      return Promise.resolve({
        manufacturer: 'Dell Inc.',
        model: 'Precision 5560',
        serviceTag: 'ABC1234',
        biosVersion: '1.47.0',
        biosDate: '05/31/2026',
      })
    return Promise.reject(`unexpected ${cmd}`)
  })
}

async function open(section: string) {
  await waitFor(() => expect(screen.getByRole('radio', { name: 'Home' }).getAttribute('aria-checked')).toBe('true'))
  await fireEvent.click(screen.getByRole('button', { name: section }))
}

const openBattery = () => open('Battery')

beforeEach(() => {
  invoke.mockReset()
})

afterEach(() => live.stop())

describe('FullWindow', () => {
  test('reports ready even when loading fails, so the error is not hidden', async () => {
    invoke.mockImplementation((cmd: string) =>
      cmd === 'get_state' ? Promise.reject('no state') : Promise.resolve([]),
    )
    render(FullWindow)
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('window_ready'))
  })

  test('cycles hidden when null', async () => {
    route(appState())
    render(FullWindow)
    await openBattery()
    expect(await screen.findByText('Wear')).toBeTruthy()
    expect(screen.queryByText('Cycles')).toBeNull()
  })

  test('cycles shown when reported', async () => {
    const s = appState()
    route({ ...s, battery: { ...s.battery!, cycles: 214 } })
    render(FullWindow)
    await openBattery()
    expect(await screen.findByText('Cycles')).toBeTruthy()
    expect(screen.getByText('214')).toBeTruthy()
  })

  test('wear shows "13.6%" for wearPct 13.6', async () => {
    const s = appState()
    route({ ...s, battery: { ...s.battery!, wearPct: 13.6 } })
    render(FullWindow)
    await openBattery()
    expect(await screen.findByText('13.6%')).toBeTruthy()
  })

  test('navigation lists every section', async () => {
    route(appState())
    render(FullWindow)
    for (const name of ['Overview', 'Battery', 'Performance', 'Sensors', 'Settings']) {
      expect(screen.getByRole('button', { name })).toBeTruthy()
    }
  })

  test('the device card shows the model and service tag', async () => {
    route(appState())
    render(FullWindow)
    expect(await screen.findByText('Precision 5560')).toBeTruthy()
    expect(screen.getByText('ABC1234')).toBeTruthy()
  })

  test('overview explains each thermal mode', async () => {
    route(appState())
    render(FullWindow)
    expect(await screen.findByText('Balanced default')).toBeTruthy()
    expect(screen.getByRole('radio', { name: 'Ultra Performance' })).toBeTruthy()
  })

  test('overview hero shows the charge and the active profile', async () => {
    route(appState())
    render(FullWindow)
    expect(await screen.findByText('On battery · Home profile')).toBeTruthy()
  })

  test('performance page holds the thermal mode', async () => {
    route(appState())
    render(FullWindow)
    await open('Performance')
    expect(screen.getByRole('heading', { level: 1, name: 'Performance' })).toBeTruthy()
    expect(screen.getByRole('radiogroup', { name: 'Thermal mode' })).toBeTruthy()
    expect(screen.queryByRole('radiogroup', { name: 'Battery profile' })).toBeNull()
  })

  test('settings page shows the version and BIOS', async () => {
    route(appState())
    render(FullWindow)
    await open('Settings')
    expect(await screen.findByText('0.1.0')).toBeTruthy()
    expect(screen.getByText('1.47.0')).toBeTruthy()
  })

  test('autostart toggle calls setAutostart(true)', async () => {
    route(appState())
    render(FullWindow)
    await open('Settings')
    const toggle = await screen.findByRole('switch', { name: 'Start at sign-in' })
    await waitFor(() => expect((toggle as HTMLButtonElement).disabled).toBe(false))
    await fireEvent.click(toggle)
    expect(invoke).toHaveBeenCalledWith('set_autostart', { enabled: true })
    await waitFor(() => expect(toggle.getAttribute('aria-checked')).toBe('true'))
  })

  test('history from getHistory is requested for 30 minutes', async () => {
    route(appState())
    render(FullWindow)
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('get_history', { minutes: 30 }))
  })
})
