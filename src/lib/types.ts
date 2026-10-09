// Mirrors the Rust types serialized by src-tauri (camelCase).

export type BatteryProfile = 'home' | 'campus' | 'storage'
export type ThermalMode = 'Optimized' | 'Cool' | 'Quiet' | 'UltraPerformance'

export type ChargeCfg =
  | { kind: 'Standard' }
  | { kind: 'Adaptive' }
  | { kind: 'PrimAcUse' }
  | { kind: 'Express' }
  | { kind: 'Custom'; start: number; stop: number }

export type ActiveProfile = { kind: 'known'; profile: BatteryProfile } | { kind: 'other'; cfg: ChargeCfg }

export interface Profiles {
  home: ChargeCfg
  campus: ChargeCfg
  storage: ChargeCfg
}

export type PowerState = 'charging' | 'discharging' | 'bypass'

export interface BatterySnapshot {
  percent: number | null
  state: PowerState
  /** Positive while charging, negative while discharging. */
  powerW: number
  remainingMwh: number
  fullMwh: number | null
  designMwh: number | null
  wearPct: number | null
  cycles: number | null
}

export interface Availability {
  cctk: boolean
  dcm: boolean
  admin: boolean
  optimizerRunning: boolean
}

export interface AppState {
  charge: ChargeCfg | null
  activeProfile: ActiveProfile | null
  thermal: ThermalMode | null
  profiles: Profiles
  battery: BatterySnapshot | null
  availability: Availability
  autostart: boolean
  optimizerWarningDismissed: boolean
}

export type GpuSnapshot =
  | { state: 'asleep' }
  | { state: 'unavailable' }
  | { state: 'active'; tempC: number; loadPct: number }

export interface FanReading {
  name: string
  rpm: number
}

export interface Telemetry {
  tsMs: number
  battery: BatterySnapshot | null
  cpu: { loadPct: number | null; tempC: number | null }
  gpu: GpuSnapshot
  fans: FanReading[]
  mem: { usedMb: number; totalMb: number } | null
  dimmC: number | null
  skinC: number | null
}

export interface HistorySample {
  tsMs: number
  cpuTemp: number | null
  cpuLoad: number | null
  gpuTemp: number | null
  batteryPct: number | null
  batteryW: number | null
}

export interface HealthEntry {
  date: string
  fullMwh: number
  designMwh: number
  cycles: number | null
}
