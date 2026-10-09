import { invoke } from '@tauri-apps/api/core'
import type { AppState, BatteryProfile, DeviceInfo, HealthEntry, HistorySample, ThermalMode } from './types'

export const api = {
  getState: () => invoke<AppState>('get_state'),
  setBatteryProfile: (profile: BatteryProfile) => invoke<AppState>('set_battery_profile', { profile }),
  setThermalMode: (mode: ThermalMode) => invoke<AppState>('set_thermal_mode', { mode }),
  getHistory: (minutes: number) => invoke<HistorySample[]>('get_history', { minutes }),
  getHealthLog: () => invoke<HealthEntry[]>('get_health_log'),
  setAutostart: (enabled: boolean) => invoke<boolean>('set_autostart', { enabled }),
  dismissOptimizerWarning: () => invoke<void>('dismiss_optimizer_warning'),
  /** Resizes the flyout to its content height (CSS px), keeping it anchored above the tray. */
  fitFlyout: (height: number) => invoke<void>('fit_flyout', { height }),
  getDeviceInfo: () => invoke<DeviceInfo>('get_device_info'),
  openFullWindow: () => invoke<void>('open_full_window'),
  /** The calling window has painted its first state; the backend shows it only now, so it never flashes blank. */
  windowReady: () => invoke<void>('window_ready'),
}
