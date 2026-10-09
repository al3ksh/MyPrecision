<script lang="ts">
  import { type Point, toPolyline } from '../lib/chart'
  import { num } from '../lib/format'

  interface Props {
    samples: Point[]
    minutes: number
    unit: string
    min?: number
    max?: number
  }

  let { samples, minutes, unit, min, max }: Props = $props()

  const HEIGHT = 120
  let canvas: HTMLCanvasElement
  let width = $state(0)

  const values = $derived(samples.flatMap((s) => (s.v == null ? [] : [s.v])))
  const vMin = $derived(min ?? Math.floor(Math.min(...values, Infinity)))
  const vMax = $derived(max ?? Math.ceil(Math.max(...values, -Infinity)))
  const latest = $derived(values.at(-1) ?? null)

  $effect(() => {
    const ctx = canvas?.getContext('2d')
    if (!ctx || !width) return
    const dpr = window.devicePixelRatio || 1
    canvas.width = Math.round(width * dpr)
    canvas.height = Math.round(HEIGHT * dpr)
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    ctx.clearRect(0, 0, width, HEIGHT)

    const style = getComputedStyle(canvas)
    ctx.strokeStyle = style.getPropertyValue('--border')
    ctx.lineWidth = 1
    for (const y of [0.5, HEIGHT / 2, HEIGHT - 0.5]) {
      ctx.beginPath()
      ctx.moveTo(0, y)
      ctx.lineTo(width, y)
      ctx.stroke()
    }

    if (!values.length) return
    const tMax = Date.now()
    const tMin = tMax - minutes * 60_000
    const lo = Number.isFinite(vMin) ? vMin : 0
    const hi = Number.isFinite(vMax) && vMax > lo ? vMax : lo + 1
    ctx.strokeStyle = style.getPropertyValue('--accent')
    ctx.lineWidth = 1.5
    ctx.lineJoin = 'round'
    for (const seg of toPolyline(samples, width, HEIGHT - 4, tMin, tMax, lo, hi)) {
      ctx.beginPath()
      seg.split(' ').forEach((pt, i) => {
        const [x, y] = pt.split(',').map(Number)
        if (i === 0) ctx.moveTo(x, y + 2)
        else ctx.lineTo(x, y + 2)
      })
      ctx.stroke()
    }
  })
</script>

<figure>
  <div class="scale num secondary">
    <span>{num(Number.isFinite(vMax) ? vMax : null)}{unit}</span>
    <span>{num(Number.isFinite(vMin) ? vMin : null)}{unit}</span>
  </div>
  <div class="plot" bind:clientWidth={width}>
    <canvas bind:this={canvas} style:height="{HEIGHT}px" aria-label="Chart, latest {num(latest)}{unit}"></canvas>
  </div>
</figure>

<style>
  figure {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 8px;
    margin: 0;
  }

  .scale {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    font-size: 11px;
    text-align: right;
  }

  .plot {
    min-width: 0;
  }

  canvas {
    display: block;
    width: 100%;
  }
</style>
