import { cleanup } from '@testing-library/svelte'
import { afterEach } from 'vitest'

// Without vitest globals Testing Library cannot register its own cleanup.
afterEach(cleanup)
