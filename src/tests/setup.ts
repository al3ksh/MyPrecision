import { cleanup } from '@testing-library/svelte'
import { afterEach } from 'vitest'

// Without vitest globals Testing Library cannot register its own cleanup.
afterEach(cleanup)

// jsdom has no ResizeObserver; Svelte's bind:clientWidth needs one.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
}

// jsdom logs "Not implemented" for canvas; charts simply skip drawing on null.
HTMLCanvasElement.prototype.getContext = (() => null) as never
