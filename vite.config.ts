import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vitest/config'

export default defineConfig(({ mode }) => ({
  plugins: [svelte()],
  // Tauri expects a fixed dev port and no screen clearing.
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  resolve: mode === 'test' ? { conditions: ['browser'] } : undefined,
  test: { environment: 'jsdom', setupFiles: ['src/tests/setup.ts'] },
}))
