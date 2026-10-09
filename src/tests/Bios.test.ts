import { fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, test, vi } from 'vitest'
import type { BiosSetting } from '../lib/types'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }))

const { default: Bios, forget } = await import('../views/full/Bios.svelte')
const { toasts } = await import('../lib/toasts.svelte')

function settings(): BiosSetting[] {
  return [
    {
      key: 'KeyboardIllumination',
      label: 'Keyboard backlight',
      description: 'Brightness',
      group: 'Keyboard',
      parent: null,
      value: 'Bright',
      kind: 'choice',
      choices: [
        { value: 'Disabled', label: 'Off' },
        { value: 'Dim', label: 'Dim' },
        { value: 'Bright', label: 'Bright' },
      ],
    },
    {
      key: 'KbdBacklightTimeoutAc',
      label: 'Backlight timeout when plugged in',
      description: 'Timeout',
      group: 'Keyboard',
      parent: 'KeyboardIllumination',
      value: '10s',
      kind: 'choice',
      choices: [
        { value: '10s', label: '10 seconds' },
        { value: '1m', label: '1 minute' },
      ],
    },
    { key: 'Camera', label: 'Camera', description: 'Built-in camera', group: 'Devices', parent: null, value: 'Enabled', kind: 'toggle' },
    { key: 'SdCard', label: 'SD card reader', description: 'Reader', group: 'Devices', parent: null, value: null, kind: 'toggle' },
    {
      key: 'AutoOnHr',
      label: 'Power-on hour',
      description: '0–23',
      group: 'Power',
      parent: null,
      value: '7',
      kind: 'number',
      min: 0,
      max: 23,
    },
  ]
}

function route(writeError?: string) {
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_bios_settings') return settings()
    if (cmd === 'set_bios_setting') {
      if (writeError) throw writeError
      return null
    }
    throw new Error(`unexpected ${cmd}`)
  })
}

const writes = () => invoke.mock.calls.filter(([cmd]) => cmd === 'set_bios_setting').map(([, a]) => a)

beforeEach(() => {
  forget()
  invoke.mockReset()
  toasts.items = []
  route()
})

describe('BIOS page', () => {
  test('a toggle asks first and writes only after confirming', async () => {
    render(Bios)
    await fireEvent.click(await screen.findByRole('switch', { name: 'Camera' }))
    const dialog = screen.getByRole('dialog')
    expect(dialog.textContent).toContain('Camera')
    expect(dialog.textContent).toContain('On')
    expect(dialog.textContent).toContain('Off')
    expect(writes()).toEqual([])

    await fireEvent.click(screen.getByRole('button', { name: 'Change' }))
    await waitFor(() => expect(writes()).toEqual([{ key: 'Camera', value: 'Disabled' }]))
    await waitFor(() => expect(screen.getByRole('switch', { name: 'Camera' }).getAttribute('aria-checked')).toBe('false'))
    expect(screen.queryByRole('dialog')).toBeNull()
  })

  test('cancelling writes nothing and keeps the shown value', async () => {
    render(Bios)
    const select = (await screen.findByLabelText('Keyboard backlight')) as HTMLSelectElement
    await fireEvent.change(select, { target: { value: 'Dim' } })
    expect(screen.getByRole('dialog').textContent).toContain('Bright')
    await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(writes()).toEqual([])
    expect(select.value).toBe('Bright')
  })

  test('Escape cancels the confirmation', async () => {
    render(Bios)
    await fireEvent.click(await screen.findByRole('switch', { name: 'Camera' }))
    await fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' })
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(writes()).toEqual([])
  })

  test('children hide while their parent is Disabled', async () => {
    render(Bios)
    await screen.findByLabelText('Backlight timeout when plugged in')
    await fireEvent.change(screen.getByLabelText('Keyboard backlight'), { target: { value: 'Disabled' } })
    await fireEvent.click(screen.getByRole('button', { name: 'Change' }))
    await waitFor(() => expect(screen.queryByLabelText('Backlight timeout when plugged in')).toBeNull())
  })

  test('a setting the BIOS did not report cannot be changed', async () => {
    render(Bios)
    const sd = await screen.findByRole('switch', { name: 'SD card reader' })
    expect((sd as HTMLButtonElement).disabled).toBe(true)
  })

  test('numbers offer exactly their range', async () => {
    render(Bios)
    const hour = (await screen.findByLabelText('Power-on hour')) as HTMLSelectElement
    expect(hour.value).toBe('7')
    expect([...hour.options].map((o) => o.value)).toEqual(Array.from({ length: 24 }, (_, i) => String(i)))
  })

  test('a rejected write shows the BIOS message and re-reads the settings', async () => {
    route('BIOS rejected the change (code 58): Setup password is required')
    render(Bios)
    await fireEvent.click(await screen.findByRole('switch', { name: 'Camera' }))
    await fireEvent.click(screen.getByRole('button', { name: 'Change' }))
    await waitFor(() =>
      expect(toasts.items.map((t) => t.text)).toEqual(['BIOS rejected the change (code 58): Setup password is required']),
    )
    await waitFor(() => expect(invoke.mock.calls.filter(([c]) => c === 'get_bios_settings')).toHaveLength(2))
    expect(screen.getByRole('switch', { name: 'Camera' }).getAttribute('aria-checked')).toBe('true')
  })

  test('a failed read offers a retry', async () => {
    invoke.mockRejectedValueOnce('Dell Command | Configure (cctk.exe) not found.')
    render(Bios)
    expect((await screen.findByText('Dell Command | Configure (cctk.exe) not found.')).textContent).toBeTruthy()
    await fireEvent.click(screen.getByRole('button', { name: /Try again/ }))
    expect(await screen.findByRole('switch', { name: 'Camera' })).toBeTruthy()
  })
})
