<script lang="ts">
  import { type Point, toPolyline } from '../lib/chart'

  /** A tiny trend line for a live card; decorative, the card states the value. */
  let { samples, min = 0, max = 100 }: { samples: Point[]; min?: number; max?: number } = $props()

  const W = 100
  const H = 28
  const t0 = $derived(samples[0]?.t ?? 0)
  const t1 = $derived(samples.at(-1)?.t ?? 1)
  const lines = $derived(toPolyline(samples, W, H - 2, t0, t1, min, max))
</script>

<svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" aria-hidden="true">
  {#each lines as points, i (i)}
    <polyline {points} transform="translate(0 1)" />
  {/each}
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: 28px;
    overflow: visible;
  }

  polyline {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.5;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }
</style>
