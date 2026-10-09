import { render, screen } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, test, vi } from 'vitest'
import AnimatedNumber from '../components/AnimatedNumber.svelte'

let now = 0
let queue: FrameRequestCallback[] = []

function runFramesUntil(endMs: number) {
  while (now < endMs) {
    now += 16
    const due = queue
    queue = []
    for (const cb of due) cb(now)
    flushSync()
  }
}

describe('AnimatedNumber', () => {
  afterEach(() => vi.unstubAllGlobals())

  test('reaches the new value within the 300 ms animation', async () => {
    now = 0
    queue = []
    vi.stubGlobal('matchMedia', () => ({ matches: false }))
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => queue.push(cb))
    vi.stubGlobal('cancelAnimationFrame', () => {})
    vi.spyOn(performance, 'now').mockImplementation(() => now)

    const { rerender } = render(AnimatedNumber, { value: 0 })
    await rerender({ value: 100 })
    runFramesUntil(320)

    expect(screen.getByText('100')).toBeTruthy()
  })
})
