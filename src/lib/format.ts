// All numbers are rendered en-US; raw f32 values from Rust carry float noise.

const fmt = (decimals: number) =>
  new Intl.NumberFormat('en-US', { minimumFractionDigits: decimals, maximumFractionDigits: decimals })

export function num(v: number | null | undefined, decimals = 0, suffix = ''): string {
  return v == null || !Number.isFinite(v) ? '—' : fmt(decimals).format(v) + suffix
}

export const temp = (c: number | null | undefined) => num(c, 0, ' °C')
export const pct = (p: number | null | undefined, decimals = 0) => num(p, decimals, '%')
export const watts = (w: number | null | undefined) => num(w == null ? null : Math.abs(w), 1, ' W')
export const wh = (mwh: number | null | undefined) => num(mwh == null ? null : mwh / 1000, 1, ' Wh')
export const rpm = (r: number | null | undefined) => num(r, 0, ' RPM')
