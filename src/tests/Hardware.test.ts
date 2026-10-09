import { fireEvent, render, screen } from '@testing-library/svelte'
import { beforeEach, describe, expect, test, vi } from 'vitest'
import type { BootReport, DriveReport, SmartLog, StorageReport } from '../lib/types'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }))
const openUrl = vi.fn()
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: (url: string) => openUrl(url) }))

const { default: Hardware, forget } = await import('../views/full/Hardware.svelte')
const { toasts } = await import('../lib/toasts.svelte')

function smart(over: Partial<SmartLog> = {}): SmartLog {
  return {
    criticalWarning: 0,
    tempC: 41,
    availableSparePct: 100,
    spareThresholdPct: 10,
    percentUsed: 3,
    bytesRead: 20.5e12,
    bytesWritten: 15.4e12,
    powerCycles: 1234,
    powerOnHours: 5678,
    unsafeShutdowns: 42,
    mediaErrors: 0,
    ...over,
  }
}

function drive(over: Partial<DriveReport> = {}): DriveReport {
  return { model: 'Micron 2300 NVMe 1TB', nvme: true, smart: smart(), yearsLeft: null, trackingSince: '2026-10-08', ...over }
}

const BOOT: BootReport = {
  boots: [
    { start: '2026-10-05T08:00:00Z', totalMs: 30000, mainPathMs: 18000, postBootMs: 12000, startupApps: 15 },
    { start: '2026-10-01T19:41:06Z', totalMs: 51577, mainPathMs: 26077, postBootMs: 25500, startupApps: 15 },
  ],
  culprits: [{ name: 'WavesSysSvc Service Application', kind: 'app', boots: 2, avgDelayMs: 12033 }],
}

let storage: StorageReport
let boot: BootReport | Error

function route() {
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_storage') return storage
    if (cmd === 'get_boot') {
      if (boot instanceof Error) throw boot.message
      return boot
    }
    throw `unexpected ${cmd}`
  })
}

beforeEach(() => {
  forget()
  invoke.mockReset()
  openUrl.mockReset()
  toasts.items = []
  storage = { drives: [drive()], volume: [1e12, 250e9] }
  boot = BOOT
  route()
})

describe('Hardware', () => {
  test('shows drive health, wear and lifetime counters', async () => {
    render(Hardware)
    expect(await screen.findByText('Micron 2300 NVMe 1TB')).toBeTruthy()
    expect(screen.getByText('Healthy')).toBeTruthy()
    expect(screen.getByText('3% of rated endurance')).toBeTruthy()
    expect(screen.getByText('15.4 TB')).toBeTruthy()
    expect(screen.getByText('5,678 h')).toBeTruthy()
    expect(screen.getByText('41 °C')).toBeTruthy()
    expect(screen.getByText(/after a week of tracking \(tracking since Oct 8, 2026\)/)).toBeTruthy()
    expect(screen.getByText('Windows (C:): 250 GB free of 1.0 TB')).toBeTruthy()
  })

  test.each([
    [{ yearsLeft: 7.4 }, {}, 'About 7 years left at your current write rate.'],
    [{ yearsLeft: 1.25 }, {}, 'About 1.3 years left at your current write rate.'],
    [{ yearsLeft: 80 }, {}, 'More than 20 years left at your current write rate.'],
    [{}, { percentUsed: 0 }, 'Less than 1% of rated endurance used.'],
    [{ yearsLeft: 0 }, { percentUsed: 104 }, 'Past its rated endurance. It may keep working, but plan a replacement.'],
  ])('forecast %#', async (d, s, text) => {
    storage = { drives: [drive({ ...d, smart: smart(s) })], volume: null }
    render(Hardware)
    expect(await screen.findByText(text)).toBeTruthy()
  })

  test('warns about a failing drive', async () => {
    storage = { drives: [drive({ smart: smart({ mediaErrors: 3 }) })], volume: null }
    render(Hardware)
    expect(await screen.findByText('3 unrecovered read errors recorded.')).toBeTruthy()
    expect(screen.queryByText('Healthy')).toBeNull()
  })

  test('explains a drive without health data', async () => {
    storage = { drives: [drive({ smart: null }), drive({ model: 'USB disk', nvme: false, smart: null })], volume: null }
    render(Hardware)
    expect(await screen.findByText('Health data needs administrator rights.')).toBeTruthy()
    expect(screen.getByText('Health data is available for NVMe drives only.')).toBeTruthy()
  })

  test('shows boot times and what slowed them down', async () => {
    render(Hardware)
    expect(await screen.findByText(/last boot, Oct 5/)).toBeTruthy()
    expect(screen.getAllByText('30.0 s')).toHaveLength(2)
    expect(screen.getByText(/18.0 s to the desktop, then 12.0 s for 15 startup apps/)).toBeTruthy()
    expect(screen.getAllByRole('listitem')).toHaveLength(2)
    expect(screen.getByText('WavesSysSvc Service Application')).toBeTruthy()
    expect(screen.getByText('App · slowed 2 of the last 2 boots')).toBeTruthy()
    expect(screen.getByText('+12.0 s')).toBeTruthy()

    await fireEvent.click(screen.getByRole('button', { name: /Manage startup apps/ }))
    expect(openUrl).toHaveBeenCalledWith('ms-settings:startupapps')
  })

  test('no boots recorded yet', async () => {
    boot = { boots: [], culprits: [] }
    render(Hardware)
    expect(await screen.findByText(/No full boots recorded yet/)).toBeTruthy()
  })

  test('a boot log error leaves the drive card working', async () => {
    boot = new Error('Startup history needs administrator rights.')
    render(Hardware)
    expect(await screen.findByText('Startup history needs administrator rights.')).toBeTruthy()
    expect(screen.getByText('Micron 2300 NVMe 1TB')).toBeTruthy()
  })

  test('refresh rereads and a failed refresh keeps the old data', async () => {
    render(Hardware)
    const refresh = await screen.findByRole('button', { name: /Refresh/ })
    boot = new Error('Boom')
    await fireEvent.click(refresh)
    await screen.findByRole('button', { name: /Refresh/ })
    expect(invoke.mock.calls.filter(([c]) => c === 'get_boot')).toHaveLength(2)
    expect(screen.getByText(/last boot/)).toBeTruthy()
    expect(toasts.items.map((t) => t.text)).toEqual(['Boom'])
  })
})
