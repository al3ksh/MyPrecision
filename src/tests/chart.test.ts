import { expect, test } from 'vitest'
import { toPolyline } from '../lib/chart'

test('toPolyline splits on null', () => {
  const pts = [1, null, 3, 4].map((v, i) => ({ t: i, v }))
  const segs = toPolyline(pts, 100, 50, 0, 3, 0, 4)
  expect(segs).toHaveLength(2)
  expect(segs[1].split(' ')).toHaveLength(2)
})

test('toPolyline maps time to x and value to inverted y', () => {
  const segs = toPolyline([{ t: 0, v: 0 }, { t: 10, v: 10 }], 100, 50, 0, 10, 0, 10)
  expect(segs).toEqual(['0,50 100,0'])
})
