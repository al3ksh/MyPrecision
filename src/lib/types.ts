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
  voltageV: number
}

export interface Availability {
  cctk: boolean
  wmi: boolean
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

/** SMBIOS identity; any field may be missing on odd firmware. */
export interface DeviceInfo {
  manufacturer: string | null
  model: string | null
  serviceTag: string | null
  biosVersion: string | null
  biosDate: string | null
}

export interface ScheduleEntry {
  /** 0 = Monday … 6 = Sunday. */
  days: number[]
  /** Local time, HH:MM. */
  time: string
  profile: BatteryProfile
}

export interface Automation {
  thermal: { enabled: boolean; onAc: ThermalMode | null; onBattery: ThermalMode | null }
  schedule: { enabled: boolean; entries: ScheduleEntry[] }
  longAc: { enabled: boolean; days: number; profile: BatteryProfile }
  notify: boolean
}

export type BiosGroup = 'Keyboard' | 'Power' | 'Devices' | 'Startup'

export interface BiosChoice {
  value: string
  label: string
}

/** One curated BIOS setting as cctk reports it; `value` is null when the BIOS did not report it. */
export type BiosSetting = {
  key: string
  label: string
  description: string
  group: BiosGroup
  /** Shown only while this setting is not `Disabled`. */
  parent: string | null
  value: string | null
} & ({ kind: 'toggle' } | { kind: 'choice'; choices: BiosChoice[] } | { kind: 'number'; min: number; max: number })
