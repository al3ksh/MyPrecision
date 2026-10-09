import { invoke } from '@tauri-apps/api/core'
import type { AppState, Automation, BatteryProfile, BiosSetting, DeviceInfo, HealthEntry, HistorySample, Telemetry, ThermalMode } from './types'

export const api = {
  getState: () => invoke<AppState>('get_state'),
  setBatteryProfile: (profile: BatteryProfile) => invoke<AppState>('set_battery_profile', { profile }),
  setThermalMode: (mode: ThermalMode) => invoke<AppState>('set_thermal_mode', { mode }),
  /** The last telemetry tick, or null while the poller has none yet. */
  getTelemetry: () => invoke<Telemetry | null>('get_telemetry'),
  getHistory: (minutes: number) => invoke<HistorySample[]>('get_history', { minutes }),
  getHealthLog: () => invoke<HealthEntry[]>('get_health_log'),
  setAutostart: (enabled: boolean) => invoke<boolean>('set_autostart', { enabled }),
  dismissOptimizerWarning: () => invoke<void>('dismiss_optimizer_warning'),
  /** Resizes the flyout to its content height (CSS px), keeping it anchored above the tray. */
  fitFlyout: (height: number) => invoke<void>('fit_flyout', { height }),
  getDeviceInfo: () => invoke<DeviceInfo>('get_device_info'),
  getAutomation: () => invoke<Automation>('get_automation'),
  setAutomation: (rules: Automation) => invoke<Automation>('set_automation', { rules }),
  /** One cctk run; takes several seconds. */
  getBiosSettings: () => invoke<BiosSetting[]>('get_bios_settings'),
  setBiosSetting: (key: string, value: string) => invoke<void>('set_bios_setting', { key, value }),
  openFullWindow: () => invoke<void>('open_full_window'),
  /** The calling window has painted its first state; the backend shows it only now, so it never flashes blank. */
  windowReady: () => invoke<void>('window_ready'),
}
