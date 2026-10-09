<script lang="ts">
  import { GLYPH, type IconName } from '../lib/icons'

  /** Decorative glyph; the surrounding control carries the accessible name. */
  let { name, size = 16 }: { name: IconName; size?: number } = $props()
</script>

<i class="icon" aria-hidden="true" style:font-size="{size}px">{GLYPH[name]}</i>

<style>
  .icon {
    display: inline-block;
    font-family: 'Segoe Fluent Icons', 'Segoe MDL2 Assets';
    font-style: normal;
    line-height: 1;
    flex: none;
  }
</style>
