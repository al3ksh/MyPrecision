import { fireEvent, render, screen, within } from '@testing-library/svelte'
import { beforeEach, describe, expect, test, vi } from 'vitest'
import type { DgpuReport, GpuApp } from '../lib/types'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }))

const { default: Power } = await import('../views/full/Power.svelte')
const { toasts } = await import('../lib/toasts.svelte')

const BRAVE = 'C:\Program Files\Brave\brave.exe'
const GAME = 'C:\Games\game.exe'

function app(over: Partial<GpuApp> = {}): GpuApp {
  return { name: 'brave', path: BRAVE, bytes: 247e6, integrated: false, ...over }
}

let dgpu: DgpuReport | null
let setError: string | null

beforeEach(() => {
  invoke.mockReset()
  toasts.items = []
  setError = null
  dgpu = { name: 'NVIDIA T1200 Laptop GPU', active: true, apps: [app()], integrated: [] }
  invoke.mockImplementation(async (cmd: string, args?: { path: string; on: boolean }) => {
    if (cmd === 'get_dgpu') return dgpu
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
})
