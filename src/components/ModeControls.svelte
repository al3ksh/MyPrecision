<script lang="ts">
  import { api } from '../lib/api'
  import type { IconName } from '../lib/icons'
  import { activeProfileLabel, chargeRangeLabel, PROFILE_LABEL, THERMAL_LABEL } from '../lib/labels'
  import { live } from '../lib/telemetry.svelte'
  import { errorText, toasts } from '../lib/toasts.svelte'
  import type { AppState, BatteryProfile, ThermalMode } from '../lib/types'
  import TileGroup from './TileGroup.svelte'

  interface Props {
    disabled?: boolean
    battery?: boolean
    thermal?: boolean
    /** Thermal tiles explain each mode (the full window has room for it). */
    describe?: boolean
  }

  /** Battery profile + thermal mode; shared by the flyout and the full window. */
  let { disabled = false, battery = true, thermal = true, describe = false }: Props = $props()

  const PROFILE_ICON: Record<BatteryProfile, IconName> = { home: 'home', campus: 'campus', storage: 'storage' }
  // Quietest to fastest, the order the tray gauge sweeps.
  const THERMAL_ORDER: ThermalMode[] = ['Quiet', 'Cool', 'Optimized', 'UltraPerformance']
  const THERMAL_ICON: Record<ThermalMode, IconName> = {
    Quiet: 'quiet',
    Cool: 'cool',
    Optimized: 'optimized',
    UltraPerformance: 'lightning',
  }
  const THERMAL_DESCRIPTION: Record<ThermalMode, string> = {
    Quiet: 'Lowest fan noise',
    Cool: 'Coolest palm rest',
    Optimized: 'Balanced default',
    UltraPerformance: 'Max clocks, louder fans',
  }
  const thermalOptions = $derived(
    THERMAL_ORDER.map((m) => ({
      value: m,
      label: THERMAL_LABEL[m],
      short: m === 'UltraPerformance' && !describe ? 'Ultra' : undefined,
      icon: THERMAL_ICON[m],
      sub: describe ? THERMAL_DESCRIPTION[m] : undefined,
    })),
  )

  // The option being written: a cctk write takes ~7 s, so the click is shown at once and the
  // indicator pulses until the BIOS answers. A failure falls back to the BIOS value.
  let pendingProfile = $state<BatteryProfile | null>(null)
  let pendingThermal = $state<ThermalMode | null>(null)

  const app = $derived(live.app)
  const profileOptions = $derived(
    (Object.keys(PROFILE_LABEL) as BatteryProfile[]).map((p) => ({
      value: p,
      label: PROFILE_LABEL[p],
      icon: PROFILE_ICON[p],
      sub: app ? chargeRangeLabel(app.profiles[p]) : '—',
    })),
  )
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

{#if battery}
  <section class="group">
    <h2>
      Battery profile
      {#if app?.activeProfile?.kind === 'other'}
        <span class="current num">{activeProfileLabel(app.activeProfile)}</span>
      {/if}
    </h2>
    <TileGroup
      label="Battery profile"
      options={profileOptions}
      value={pendingProfile ?? profileValue}
      disabled={disabled || !app}
      busy={pendingProfile !== null}
      onchange={setProfile}
    />
  </section>
{/if}

{#if thermal}
  <section class="group">
    <h2>Thermal mode</h2>
    <TileGroup
      label="Thermal mode"
      options={thermalOptions}
      value={pendingThermal ?? app?.thermal ?? null}
      disabled={disabled || !app}
      busy={pendingThermal !== null}
      onchange={setThermal}
    />
  </section>
{/if}

<style>
  .group {
    display: grid;
    gap: 8px;
  }

  h2 {
    display: flex;
    justify-content: space-between;
    margin: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-2);
  }

  .current {
    color: var(--text);
  }
</style>
