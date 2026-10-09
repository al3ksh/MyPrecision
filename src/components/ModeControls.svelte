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

  let profileBusy = $state(false)
  let thermalBusy = $state(false)

  const app = $derived(live.app)
  const profileValue = $derived(app?.activeProfile?.kind === 'known' ? app.activeProfile.profile : null)

  async function run(call: () => Promise<AppState>, setBusy: (b: boolean) => void) {
    setBusy(true)
    try {
      live.app = await call()
    } catch (e) {
      toasts.push(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const setProfile = (v: string) => run(() => api.setBatteryProfile(v as BatteryProfile), (b) => (profileBusy = b))
  const setThermal = (v: string) => run(() => api.setThermalMode(v as ThermalMode), (b) => (thermalBusy = b))
</script>

<section class="group">
  <h2>Battery profile</h2>
  <SegmentedControl
    label="Battery profile"
    options={profileOptions}
    value={profileValue}
    disabled={disabled || !app}
    busy={profileBusy}
    onchange={setProfile}
  />
</section>

<section class="group">
  <h2>Thermal mode</h2>
  <SegmentedControl
    label="Thermal mode"
    options={thermalOptions}
    value={app?.thermal ?? null}
    disabled={disabled || !app}
    busy={thermalBusy}
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
