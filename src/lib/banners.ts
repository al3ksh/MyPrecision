import type { AppState } from './types'

export type BannerId = 'noCctk' | 'noAdmin' | 'noWmi' | 'noDcm' | 'optimizer'

export interface BannerText {
  title: string
  body: string
  action?: string
  /** Opened by the action instead of dismissing the banner. */
  url?: string
}

export const BANNER_TEXT: Record<BannerId, BannerText> = {
  noAdmin: { title: 'Not running as administrator', body: 'Restart MyPrecision as administrator.' },
  noCctk: {
    title: 'Dell Command | Configure not found',
    body: 'Battery profiles, thermal modes and BIOS settings need Dell Command | Configure, a free Dell tool. Install it, then restart MyPrecision.',
    action: 'Download',
    url: 'https://www.dell.com/support/kbdoc/en-us/000178000/dell-command-configure',
  },
  optimizer: {
    title: 'Dell Optimizer is running',
    body: 'Make sure Dynamic Charge is off — otherwise Optimizer may override your profile.',
    action: 'Don’t show again',
  },
  noWmi: {
    title: 'Windows sensors unavailable',
    body: 'WMI could not be reached, so battery and temperature readings are missing. Restarting Windows usually fixes this.',
  },
  noDcm: {
    title: 'Dell Command | Monitor not found',
    body: 'Fan speeds and some temperatures need Dell Command | Monitor, a free Dell tool. Install it, then restart MyPrecision.',
    action: 'Download',
    url: 'https://www.dell.com/support/kbdoc/en-us/000177080/dell-command-monitor',
  },
}

/** Active banners, most severe first. */
export function bannersFor(s: AppState): BannerId[] {
  const a = s.availability
  const ids: BannerId[] = []
  if (!a.admin) ids.push('noAdmin')
  if (!a.cctk) ids.push('noCctk')
  if (a.optimizerRunning && !s.optimizerWarningDismissed) ids.push('optimizer')
  // Without WMI, Dell Command | Monitor cannot be queried either: report the cause, not the symptom.
  if (!a.wmi) ids.push('noWmi')
  else if (!a.dcm) ids.push('noDcm')
  return ids
}
