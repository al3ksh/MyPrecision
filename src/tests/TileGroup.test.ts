import { fireEvent, render, screen } from '@testing-library/svelte'
import { describe, expect, test, vi } from 'vitest'
import TileGroup from '../components/TileGroup.svelte'

const options = [
  { value: 'home', label: 'Home' },
  { value: 'campus', label: 'Campus' },
  { value: 'storage', label: 'Storage' },
]

describe('TileGroup', () => {
  test('renders options as radios with aria-checked', () => {
    render(TileGroup, { options, value: 'home', onchange: () => {} })
    const radios = screen.getAllByRole('radio')
    expect(radios).toHaveLength(3)
    expect(radios.map((r) => r.getAttribute('aria-checked'))).toEqual(['true', 'false', 'false'])
    expect(screen.getByRole('radiogroup')).toBeTruthy()
  })

  test('click calls onchange with value', async () => {
    const onchange = vi.fn()
    render(TileGroup, { options, value: 'home', onchange })
    await fireEvent.click(screen.getByRole('radio', { name: 'Storage' }))
    expect(onchange).toHaveBeenCalledWith('storage')
  })

  test('null value → no option checked', () => {
    render(TileGroup, { options, value: null, onchange: () => {} })
    expect(screen.getAllByRole('radio').every((r) => r.getAttribute('aria-checked') === 'false')).toBe(true)
  })

  test('disabled/busy → click does not call onchange', async () => {
    const onchange = vi.fn()
    render(TileGroup, { options, value: 'home', disabled: true, onchange })
    await fireEvent.click(screen.getByRole('radio', { name: 'Campus' }))
    const busy = render(TileGroup, { options, value: 'home', busy: true, onchange })
    await fireEvent.click(busy.getAllByRole('radio', { name: 'Campus' })[0])
    expect(onchange).not.toHaveBeenCalled()
  })

  test('arrows only move focus — each BIOS write needs Enter, Space or a click', async () => {
    const onchange = vi.fn()
    render(TileGroup, { options, value: 'home', onchange })
    const home = screen.getByRole('radio', { name: 'Home' })
    home.focus()
    await fireEvent.keyDown(home, { key: 'ArrowRight' })
    await fireEvent.keyDown(screen.getByRole('radio', { name: 'Campus' }), { key: 'ArrowRight' })
    expect(document.activeElement).toBe(screen.getByRole('radio', { name: 'Storage' }))
    expect(onchange).not.toHaveBeenCalled()
    await fireEvent.click(document.activeElement as HTMLElement)
    expect(onchange).toHaveBeenCalledExactlyOnceWith('storage')
  })

  test('ArrowLeft wraps from the first option to the last', async () => {
    render(TileGroup, { options, value: 'home', onchange: () => {} })
    await fireEvent.keyDown(screen.getByRole('radio', { name: 'Home' }), { key: 'ArrowLeft' })
    expect(document.activeElement).toBe(screen.getByRole('radio', { name: 'Storage' }))
  })
})

describe('TileGroup tiles', () => {
  const rich = [
    { value: 'home', label: 'Home', icon: 'home' as const, sub: '75–80%' },
    { value: 'ultra', label: 'Ultra Performance', short: 'Ultra', icon: 'lightning' as const },
  ]

  test('tile is named by its full label even when it shows a short label and a subtitle', () => {
    render(TileGroup, { options: rich, value: 'home', onchange: () => {} })
    expect(screen.getByRole('radio', { name: 'Home' })).toBeTruthy()
    const ultra = screen.getByRole('radio', { name: 'Ultra Performance' })
    expect(ultra.textContent).toContain('Ultra')
    expect(ultra.textContent).not.toContain('Performance')
  })

  test('subtitle is shown and the icon is hidden from assistive tech', () => {
    render(TileGroup, { options: rich, value: 'home', onchange: () => {} })
    const home = screen.getByRole('radio', { name: 'Home' })
    expect(home.textContent).toContain('75–80%')
    expect(home.querySelector('[aria-hidden="true"]')).toBeTruthy()
  })
})
