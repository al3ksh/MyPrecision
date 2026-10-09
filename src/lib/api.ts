import { invoke } from '@tauri-apps/api/core'
import type { AppState, BatteryProfile, HealthEntry, HistorySample, ThermalMode } from './types'

export const api = {
  getState: () => invoke<AppState>('get_state'),
  setBatteryProfile: (profile: BatteryProfile) => invoke<AppState>('set_battery_profile', { profile }),
  setThermalMode: (mode: ThermalMode) => invoke<AppState>('set_thermal_mode', { mode }),
  getHistory: (minutes: number) => invoke<HistorySample[]>('get_history', { minutes }),
  getHealthLog: () => invoke<HealthEntry[]>('get_health_log'),
  setAutostart: (enabled: boolean) => invoke<boolean>('set_autostart', { enabled }),
  dismissOptimizerWarning: () => invoke<void>('dismiss_optimizer_warning'),
  openFullWindow: () => invoke<void>('open_full_window'),
}
