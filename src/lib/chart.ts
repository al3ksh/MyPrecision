export interface Point {
  t: number
  v: number | null
}

/** Polyline strings "x,y x,y …", one per run of non-null values (gaps stay gaps). */
export function toPolyline(
  pts: Point[],
  w: number,
  h: number,
  tMin: number,
  tMax: number,
  vMin: number,
  vMax: number,
): string[] {
  const tSpan = tMax - tMin || 1
  const vSpan = vMax - vMin || 1
  const segments: string[] = []
  let current: string[] = []
  for (const p of pts) {
    if (p.v == null) {
      if (current.length) segments.push(current.join(' '))
      current = []
      continue
    }
    const x = ((p.t - tMin) / tSpan) * w
    const y = h - ((Math.min(Math.max(p.v, vMin), vMax) - vMin) / vSpan) * h
    current.push(`${round(x)},${round(y)}`)
  }
  if (current.length) segments.push(current.join(' '))
  return segments
}

const round = (n: number) => Math.round(n * 10) / 10
