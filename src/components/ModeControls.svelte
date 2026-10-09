<script lang="ts">
  import { api } from '../lib/api'
  import { PROFILE_LABEL, THERMAL_LABEL } from '../lib/labels'
  import { live } from '../lib/telemetry.svelte'
  import { errorText, toasts } from '../lib/toasts.svelte'
  import type { AppState, BatteryProfile, ThermalMode } from '../lib/types'
  import SegmentedControl from './SegmentedControl.svelte'

  /** Battery profile + thermal mode; shared by the flyout and the full window. */
  let { disabled = false }: { disabled?: boolean } = $props()

  const profileOptions = (Object.keys(PROFILE_LABEL) as BatteryProfile[]).map((p) => ({
    value: p,
    label: PROFILE_LABEL[p],
  }))
  const thermalOptions = (Object.keys(THERMAL_LABEL) as ThermalMode[]).map((m) => ({
    value: m,
    label: THERMAL_LABEL[m],
  }))

  // The option being written: a cctk write takes ~7 s, so the click is shown at once and the
  // indicator pulses until the BIOS answers. A failure falls back to the BIOS value.
  let pendingProfile = $state<BatteryProfile | null>(null)
  let pendingThermal = $state<ThermalMode | null>(null)

  const app = $derived(live.app)
  const profileValue = $derived(app?.activeProfile?.kind === 'known' ? app.activeProfile.profile : null)

  async function run<T>(value: T, call: (v: T) => Promise<AppState>, setPending: (v: T | null) => void) {
    setPending(value)
    try {
      live.app = await call(value)
    } catch (e) {
      toasts.push(errorText(e))
    } finally {
      setPending(null)
    }
  }

  const setProfile = (v: string) =>
    run(v as BatteryProfile, api.setBatteryProfile, (p) => (pendingProfile = p))
  const setThermal = (v: string) => run(v as ThermalMode, api.setThermalMode, (m) => (pendingThermal = m))
</script>

<section class="group">
  <h2>Battery profile</h2>
  <SegmentedControl
    label="Battery profile"
    options={profileOptions}
    value={pendingProfile ?? profileValue}
    disabled={disabled || !app}
    busy={pendingProfile !== null}
    onchange={setProfile}
  />
</section>

<section class="group">
  <h2>Thermal mode</h2>
  <SegmentedControl
    label="Thermal mode"
    options={thermalOptions}
    value={pendingThermal ?? app?.thermal ?? null}
    disabled={disabled || !app}
    busy={pendingThermal !== null}
    onchange={setThermal}
  />
</section>

<style>
  .group {
    display: grid;
    gap: 6px;
  }

  h2 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }
</style>
