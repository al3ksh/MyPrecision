import { fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, test, vi } from 'vitest'
import type { Automation as Rules } from '../lib/types'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }))

const { default: Automation } = await import('../views/full/Automation.svelte')
const { toasts } = await import('../lib/toasts.svelte')

function rules(over: Partial<Rules> = {}): Rules {
  return {
    thermal: { enabled: false, onAc: null, onBattery: null },
    schedule: { enabled: false, entries: [] },
    longAc: { enabled: false, days: 3, profile: 'storage' },
    notify: true,
    ...over,
  }
}

let stored: Rules
function route(saveError?: string) {
  invoke.mockImplementation(async (cmd: string, args?: { rules: Rules }) => {
    if (cmd === 'get_automation') return structuredClone(stored)
    if (cmd === 'set_automation') {
      if (saveError) throw saveError
      stored = structuredClone(args!.rules)
      return stored
    }
    throw new Error(`unexpected ${cmd}`)
  })
}

function saved(): Rules {
  const calls = invoke.mock.calls.filter(([cmd]) => cmd === 'set_automation')
  return calls.at(-1)![1].rules
}

beforeEach(() => {
  invoke.mockReset()
  toasts.items = []
  stored = rules()
  route()
})

describe('Automation page', () => {
  test('turning on thermal rules saves them and reveals the mode pickers', async () => {
    render(Automation)
    await fireEvent.click(await screen.findByRole('switch', { name: 'Thermal mode by power source' }))
    await waitFor(() => expect(saved().thermal.enabled).toBe(true))

    await fireEvent.change(screen.getByLabelText('Plugged in'), { target: { value: 'UltraPerformance' } })
    await waitFor(() => expect(saved().thermal.onAc).toBe('UltraPerformance'))

    await fireEvent.change(screen.getByLabelText('Plugged in'), { target: { value: '' } })
    await waitFor(() => expect(saved().thermal.onAc).toBeNull())
  })

  test('adding a schedule entry defaults to weekdays at 08:00 on Campus', async () => {
    stored = rules({ schedule: { enabled: true, entries: [] } })
    render(Automation)
    await fireEvent.click(await screen.findByRole('button', { name: /Add entry/ }))
    await waitFor(() =>
      expect(saved().schedule.entries).toEqual([{ days: [0, 1, 2, 3, 4], time: '08:00', profile: 'campus' }]),
    )
  })

  test('day chips toggle, and the last day of an entry cannot be removed', async () => {
    stored = rules({ schedule: { enabled: true, entries: [{ days: [0], time: '07:30', profile: 'home' }] } })
    render(Automation)
    const mon = await screen.findByRole('button', { name: 'Mon' })
    expect((mon as HTMLButtonElement).disabled).toBe(true)

    await fireEvent.click(screen.getByRole('button', { name: 'Sat' }))
    await waitFor(() => expect(saved().schedule.entries[0].days).toEqual([0, 5]))
    expect((screen.getByRole('button', { name: 'Mon' }) as HTMLButtonElement).disabled).toBe(false)
  })

  test('removing an entry saves the shorter list', async () => {
    stored = rules({
      schedule: {
        enabled: true,
        entries: [
          { days: [0], time: '07:30', profile: 'home' },
          { days: [5], time: '10:00', profile: 'storage' },
        ],
      },
    })
    render(Automation)
    await fireEvent.click(await screen.findByRole('button', { name: 'Remove entry 1' }))
    await waitFor(() => expect(saved().schedule.entries).toEqual([{ days: [5], time: '10:00', profile: 'storage' }]))
  })

  test('long-AC days outside 1–60 are not saved', async () => {
    stored = rules({ longAc: { enabled: true, days: 3, profile: 'storage' } })
    render(Automation)
    const days = await screen.findByLabelText('After days on AC')
    await fireEvent.change(days, { target: { value: '0' } })
    expect(invoke).not.toHaveBeenCalledWith('set_automation', expect.anything())
    expect((days as HTMLInputElement).value).toBe('3')

    await fireEvent.change(days, { target: { value: '7' } })
    await waitFor(() => expect(saved().longAc.days).toBe(7))
  })

  test('a failed save shows the error and reloads the stored rules', async () => {
    route('Could not save settings: disk full')
    render(Automation)
    const notify = await screen.findByRole('switch', { name: 'Notifications' })
    await fireEvent.click(notify)
    await waitFor(() => expect(toasts.items.map((t) => t.text)).toEqual(['Could not save settings: disk full']))
    await waitFor(() => expect(screen.getByRole('switch', { name: 'Notifications' }).getAttribute('aria-checked')).toBe('true'))
  })
})
