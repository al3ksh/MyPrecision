import { expect, test } from 'vitest'
import { activeProfileLabel, chargeCfgLabel, chargeRangeLabel } from '../lib/labels'

test('activeProfileLabel Other Custom', () =>
  expect(activeProfileLabel({ kind: 'other', cfg: { kind: 'Custom', start: 60, stop: 90 } })).toBe('Custom (60–90%)'))

test('activeProfileLabel known and other BIOS modes', () => {
  expect(activeProfileLabel({ kind: 'known', profile: 'campus' })).toBe('Campus')
  expect(activeProfileLabel({ kind: 'other', cfg: { kind: 'Adaptive' } })).toBe('Adaptive')
  expect(activeProfileLabel({ kind: 'other', cfg: { kind: 'Express' } })).toBe('Express')
  expect(activeProfileLabel({ kind: 'other', cfg: { kind: 'PrimAcUse' } })).toBe('Primarily AC')
  expect(activeProfileLabel(null)).toBe('—')
})

test('chargeCfgLabel', () => {
  expect(chargeCfgLabel({ kind: 'Custom', start: 75, stop: 80 })).toBe('75–80%')
  expect(chargeCfgLabel({ kind: 'Standard' })).toBe('Standard (up to 100%)')
})

test('chargeRangeLabel is short enough for a tile subtitle', () => {
  expect(chargeRangeLabel({ kind: 'Custom', start: 75, stop: 80 })).toBe('75–80%')
  expect(chargeRangeLabel({ kind: 'Standard' })).toBe('Up to 100%')
  expect(chargeRangeLabel({ kind: 'Adaptive' })).toBe('Adaptive')
})
