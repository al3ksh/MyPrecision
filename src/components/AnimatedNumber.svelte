<script lang="ts">
  import { untrack } from 'svelte'
  import { num } from '../lib/format'

  interface Props {
    value: number | null
    decimals?: number
    suffix?: string
  }

  let { value, decimals = 0, suffix = '' }: Props = $props()

  const DURATION = 300
  let shown = $state<number | null>(null)
  let frame = 0

  $effect(() => {
    const target = value
    // Only a new value restarts the animation; frames write `shown` without re-running this.
    const from = untrack(() => shown)
    cancelAnimationFrame(frame)
    const reduced = globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches
    if (target == null || from == null || reduced) {
      shown = target
      return
    }
    const start = performance.now()
    const step = (now: number) => {
      const k = Math.min(1, (now - start) / DURATION)
      const eased = 1 - (1 - k) ** 3
      shown = from + (target - from) * eased
      if (k < 1) frame = requestAnimationFrame(step)
    }
    frame = requestAnimationFrame(step)
    return () => cancelAnimationFrame(frame)
  })
</script>

<span class="num">{num(shown, decimals, suffix)}</span>
