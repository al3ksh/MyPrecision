<script lang="ts">
  import Card from '../../components/Card.svelte'
  import ModeControls from '../../components/ModeControls.svelte'
  import { pct, watts } from '../../lib/format'
  import { activeProfileLabel, chargeCfgLabel, PROFILE_LABEL } from '../../lib/labels'
  import { live } from '../../lib/telemetry.svelte'
  import type { BatteryProfile } from '../../lib/types'

  const DESCRIPTION: Record<BatteryProfile, string> = {
    home: 'Holds 75–80% — ideal when plugged in at a desk',
    campus: 'Charges to 100% before you head out',
    storage: '50–60% for long-term storage',
  }

  const app = $derived(live.app)
  const battery = $derived(live.telemetry?.battery ?? app?.battery ?? null)
  const active = $derived(app?.activeProfile?.kind === 'known' ? app.activeProfile.profile : null)
</script>

<div class="stack">
  <Card>
    <div class="summary">
      <span class="big num">{pct(battery?.percent)}</span>
      <div class="meta">
        <span class="profile">{activeProfileLabel(app?.activeProfile ?? null)}</span>
        <span class="secondary num">
          {#if !battery}
            —
          {:else if battery.state === 'charging'}
            Charging {watts(battery.powerW)}
          {:else if battery.state === 'discharging'}
            Discharging {watts(battery.powerW)}
          {:else}
            Plugged in — battery bypassed
          {/if}
        </span>
      </div>
    </div>
  </Card>

  <Card>
    <div class="controls">
      <ModeControls disabled={app ? !app.availability.cctk : false} />
    </div>
  </Card>

  <Card title="Profiles">
    <ul>
      {#each Object.keys(PROFILE_LABEL) as BatteryProfile[] as p (p)}
        <li class:active={active === p}>
          <div>
            <span class="name">{PROFILE_LABEL[p]}</span>
            <span class="secondary">{DESCRIPTION[p]}</span>
          </div>
          <span class="secondary num">{app ? chargeCfgLabel(app.profiles[p]) : '—'}</span>
        </li>
      {/each}
    </ul>
  </Card>
</div>

<style>
  .stack {
    display: grid;
    gap: 12px;
    max-width: 720px;
  }

  .summary {
    display: flex;
    align-items: center;
    gap: 18px;
  }

  .big {
    font-size: 44px;
    font-weight: 500;
    line-height: 1;
  }

  .meta {
    display: grid;
    gap: 2px;
  }

  .profile {
    font-weight: 500;
  }

  .controls {
    display: grid;
    gap: 14px;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 2px;
  }

  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: var(--r-ctl);
  }

  li.active {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  li div {
    display: grid;
    gap: 2px;
  }

  .name {
    font-weight: 500;
  }
</style>
