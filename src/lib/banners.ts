import type { AppState } from './types'

export type BannerId = 'noCctk' | 'noAdmin' | 'noDcm' | 'optimizer'

export const BANNER_TEXT: Record<BannerId, { title: string; body: string; action?: string }> = {
  noAdmin: { title: 'Not running as administrator', body: 'Restart MyPrecision as administrator.' },
  noCctk: {
    title: 'Dell Command | Configure not found',
    body: 'Mode switching is unavailable. Install Dell Command | Configure.',
  },
  optimizer: {
    title: 'Dell Optimizer is running',
    body: 'Make sure Dynamic Charge is off — otherwise Optimizer may override your profile.',
    action: 'Don’t show again',
  },
  noDcm: {
    title: 'Dell Command | Monitor not found',
    body: 'CPU temperature and fan speeds are unavailable.',
  },
}

/** Active banners, most severe first. */
export function bannersFor(s: AppState): BannerId[] {
  const a = s.availability
  const ids: BannerId[] = []
  if (!a.admin) ids.push('noAdmin')
  if (!a.cctk) ids.push('noCctk')
  if (a.optimizerRunning && !s.optimizerWarningDismissed) ids.push('optimizer')
  if (!a.dcm) ids.push('noDcm')
  return ids
}
