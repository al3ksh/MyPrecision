import { fireEvent, render, screen } from '@testing-library/svelte'
import { describe, expect, test, vi } from 'vitest'
import SegmentedControl from '../components/SegmentedControl.svelte'

const options = [
  { value: 'home', label: 'Home' },
  { value: 'campus', label: 'Campus' },
  { value: 'storage', label: 'Storage' },
]

describe('SegmentedControl', () => {
  test('renders options as radios with aria-checked', () => {
    render(SegmentedControl, { options, value: 'home', onchange: () => {} })
    const radios = screen.getAllByRole('radio')
    expect(radios).toHaveLength(3)
    expect(radios.map((r) => r.getAttribute('aria-checked'))).toEqual(['true', 'false', 'false'])
    expect(screen.getByRole('radiogroup')).toBeTruthy()
  })

  test('click calls onchange with value', async () => {
    const onchange = vi.fn()
    render(SegmentedControl, { options, value: 'home', onchange })
    await fireEvent.click(screen.getByRole('radio', { name: 'Storage' }))
    expect(onchange).toHaveBeenCalledWith('storage')
  })

  test('null value → no option checked, indicator hidden', () => {
    const { container } = render(SegmentedControl, { options, value: null, onchange: () => {} })
    expect(screen.getAllByRole('radio').every((r) => r.getAttribute('aria-checked') === 'false')).toBe(true)
    expect(container.querySelector('.indicator')).toBeNull()
  })

  test('disabled/busy → click does not call onchange', async () => {
    const onchange = vi.fn()
    render(SegmentedControl, { options, value: 'home', disabled: true, onchange })
    await fireEvent.click(screen.getByRole('radio', { name: 'Campus' }))
    const busy = render(SegmentedControl, { options, value: 'home', busy: true, onchange })
    await fireEvent.click(busy.getAllByRole('radio', { name: 'Campus' })[0])
    expect(onchange).not.toHaveBeenCalled()
  })

  test('ArrowRight moves to next option and calls onchange', async () => {
    const onchange = vi.fn()
    render(SegmentedControl, { options, value: 'home', onchange })
    await fireEvent.keyDown(screen.getByRole('radio', { name: 'Home' }), { key: 'ArrowRight' })
    expect(onchange).toHaveBeenCalledWith('campus')
  })
})
