import { fireEvent, render, screen, within } from '@testing-library/svelte'
import { beforeEach, describe, expect, test, vi } from 'vitest'
import type { DgpuReport, EnergyReport, GpuApp, SleepReport } from '../lib/types'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }))

const { default: Power, forget } = await import('../views/full/Power.svelte')
const { toasts } = await import('../lib/toasts.svelte')

const BRAVE = 'C:\Program Files\Brave\brave.exe'
const GAME = 'C:\Games\game.exe'

function app(over: Partial<GpuApp> = {}): GpuApp {
  return { name: 'brave', path: BRAVE, bytes: 247e6, integrated: false, ...over }
}

let dgpu: DgpuReport | null
let setError: string | null
let energy: EnergyReport | string
let sleep: SleepReport | string

const ADMIN = 'Battery history needs administrator rights.'

beforeEach(() => {
  forget()
  invoke.mockReset()
  toasts.items = []
  setError = null
  dgpu = { name: 'NVIDIA T1200 Laptop GPU', active: true, apps: [app()], integrated: [] }
  energy = {
    day: [
      { name: 'Claude', mwh: 9000, screenOffMwh: 0 },
      { name: 'System', mwh: 3000, screenOffMwh: 2000 },
    ],
    week: [{ name: 'brave', mwh: 20000, screenOffMwh: 0 }],
    dayTotalMwh: 12000,
    weekTotalMwh: 40000,
  }
  sleep = {
    sessions: [
      { start: '2026-10-05T20:00:00Z', minutes: 600, drainedMwh: 5923, fullMwh: 59228, deepPct: 99, blocker: null },
      { start: '2026-10-05T01:00:00Z', minutes: 60, drainedMwh: 2961, fullMwh: 59228, deepPct: 82, blocker: 'Windows Update' },
    ],
  }
  invoke.mockImplementation(async (cmd: string, args?: { path: string; on: boolean }) => {
    if (cmd === 'get_dgpu') return dgpu
    if (cmd === 'get_app_energy') {
      if (typeof energy === 'string') throw energy
      return energy
    }
    if (cmd === 'get_sleep') {
      if (typeof sleep === 'string') throw sleep
      return sleep
    }
    if (cmd === 'set_integrated_gpu') {
      if (setError) throw setError
      const a = dgpu!.apps.find((x) => x.path === args!.path)
      if (a) a.integrated = args!.on
      dgpu!.integrated = args!.on ? [app({ path: args!.path, bytes: 0, integrated: true })] : []
      return
    }
    throw `unexpected ${cmd}`
  })
})

describe('Power', () => {
  test('names the apps keeping the discrete GPU awake', async () => {
    render(Power)
    expect(await screen.findByText('NVIDIA T1200 Laptop GPU')).toBeTruthy()
    expect(screen.getByText('Active')).toBeTruthy()
    expect(screen.getByText(/1 app keeps it awake/)).toBeTruthy()
    const row = screen.getByText('brave').closest('.row') as HTMLElement
    expect(within(row).getByText('247 MB on the GPU')).toBeTruthy()
  })

  test('moving an app to the integrated GPU, then undoing it', async () => {
    render(Power)
    await fireEvent.click(await screen.findByRole('button', { name: 'Use integrated graphics' }))
    expect(invoke).toHaveBeenCalledWith('set_integrated_gpu', { path: BRAVE, on: true })
    expect(await screen.findByText(/Restart brave to switch/)).toBeTruthy()

    await fireEvent.click(screen.getAllByRole('button', { name: 'Undo' })[0])
    expect(invoke).toHaveBeenCalledWith('set_integrated_gpu', { path: BRAVE, on: false })
    expect(await screen.findByRole('button', { name: 'Use integrated graphics' })).toBeTruthy()
  })

  test('a sleeping GPU lists the apps already moved off it', async () => {
    dgpu = { name: 'NVIDIA RTX A2000', active: false, apps: [], integrated: [app({ name: 'game', path: GAME, bytes: 0, integrated: true })] }
    render(Power)
    expect(await screen.findByText('Sleeping')).toBeTruthy()
    expect(screen.getByText(/No app is using it/)).toBeTruthy()
    expect(screen.getByText('game')).toBeTruthy()
    expect(screen.getByText(GAME)).toBeTruthy()
    await fireEvent.click(screen.getByRole('button', { name: 'Undo' }))
    expect(invoke).toHaveBeenCalledWith('set_integrated_gpu', { path: GAME, on: false })
  })

  test('integrated graphics only hides the GPU card', async () => {
    dgpu = null
    render(Power)
    expect(await screen.findByText(/integrated graphics only/)).toBeTruthy()
    expect(screen.queryByText('Active')).toBeNull()
  })

  test('a failed switch shows why', async () => {
    setError = "Windows didn't accept the graphics setting."
    render(Power)
    await fireEvent.click(await screen.findByRole('button', { name: 'Use integrated graphics' }))
    await vi.waitFor(() => expect(toasts.items.map((t) => t.text)).toEqual([setError]))
  })

  test('battery use by app, for the last day and then the week', async () => {
    render(Power)
    const claude = (await screen.findByText('Claude')).closest('.row') as HTMLElement
    expect(within(claude).getByText('9.0 Wh')).toBeTruthy()
    expect(within(claude).getByText('75%')).toBeTruthy()
    const system = screen.getByText('System').closest('.row') as HTMLElement
    expect(within(system).getByText('2.0 Wh with the screen off')).toBeTruthy()

    await fireEvent.click(screen.getByRole('button', { name: '7 days' }))
    const brave = screen.getByText('brave', { selector: '.energy span' }).closest('.row') as HTMLElement
    expect(within(brave).getByText('20.0 Wh')).toBeTruthy()
    expect(within(brave).getByText('50%')).toBeTruthy()
    expect(screen.queryByText('Claude')).toBeNull()
  })

  test('no time on battery says so', async () => {
    energy = { day: [], week: [], dayTotalMwh: 0, weekTotalMwh: 0 }
    render(Power)
    expect(await screen.findByText('No time on battery in this period.')).toBeTruthy()
  })

  test('sleep drain per session and what kept it awake', async () => {
    render(Power)
    expect(await screen.findByText(/loses about 1.4% an hour asleep on battery. That's normal./)).toBeTruthy()
    const night = screen.getByText(/10 h asleep/).closest('.row') as HTMLElement
    expect(within(night).getByText('10% used')).toBeTruthy()
    expect(within(night).getByText('1.0%/h')).toBeTruthy()
    expect(within(night).getByText(/99% deep sleep/)).toBeTruthy()
    const nap = screen.getByText(/1 h asleep/).closest('.row') as HTMLElement
    expect(within(nap).getByText('5.0%/h')).toBeTruthy()
    expect(within(nap).getByText('Kept awake by Windows Update')).toBeTruthy()
  })

  test('heavy sleep drain is called out', async () => {
    sleep = {
      sessions: [
        { start: '2026-10-05T20:00:00Z', minutes: 120, drainedMwh: 5923, fullMwh: 59228, deepPct: 70, blocker: 'NVIDIA T1200' },
      ],
    }
    render(Power)
    expect(await screen.findByText(/loses about 5.0% an hour asleep on battery. That's more than it should/)).toBeTruthy()
  })

  test('without administrator rights both battery sections say so', async () => {
    energy = sleep = ADMIN
    render(Power)
    await vi.waitFor(() => expect(screen.getAllByText(ADMIN)).toHaveLength(2))
    expect(screen.getByText('NVIDIA T1200 Laptop GPU')).toBeTruthy()
  })

  test('refresh reads both reports again', async () => {
    render(Power)
    await fireEvent.click(await screen.findByRole('button', { name: 'Refresh' }))
    await vi.waitFor(() => expect(invoke.mock.calls.filter(([c]) => c === 'get_sleep')).toHaveLength(2))
    expect(invoke.mock.calls.filter(([c]) => c === 'get_app_energy')).toHaveLength(2)
  })
})
