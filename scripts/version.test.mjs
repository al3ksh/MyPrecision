import { describe, expect, it } from 'vitest'
import { bumpCargoLock, bumpCargoToml, bumpJson, bumpPackageLock, releaseChangelog } from './version.mjs'

describe('bumpCargoToml', () => {
  it('changes the package version only', () => {
    const toml = '[package]\nname = "mystation"\nversion = "0.5.0"\n\n[dependencies]\nserde = { version = "1" }\nchrono = "0.5.0"\n'
    expect(bumpCargoToml(toml, '0.5.1')).toBe(toml.replace('version = "0.5.0"', 'version = "0.5.1"'))
  })

  it('fails when there is no package version', () => {
    expect(() => bumpCargoToml('[workspace]\n', '0.5.1')).toThrow()
  })
})

describe('bumpCargoLock', () => {
  const lock = `[[package]]
name = "mystation"
version = "0.5.0"
dependencies = [
 "mystation-core",
]

[[package]]
name = "mystation-core"
version = "0.5.0"

[[package]]
name = "other"
version = "0.5.0"
`

  it('changes the named crates and leaves the rest', () => {
    const out = bumpCargoLock(lock, ['mystation', 'mystation-core'], '0.5.1')
    expect(out.match(/version = "0\.5\.1"/g)).toHaveLength(2)
    expect(out).toContain('name = "other"\nversion = "0.5.0"')
  })

  it('fails when a crate is missing', () => {
    expect(() => bumpCargoLock(lock, ['nope'], '0.5.1')).toThrow()
  })
})

describe('bumpJson', () => {
  it('changes the top-level version and keeps the formatting', () => {
    const json = '{\n  "productName": "MyStation",\n  "version": "0.5.0",\n  "plugins": { "updater": { "version": "2" } }\n}\n'
    expect(bumpJson(json, '0.5.1')).toBe(json.replace('"version": "0.5.0"', '"version": "0.5.1"'))
  })
})

describe('bumpPackageLock', () => {
  it('changes the root and the root package, not dependencies', () => {
    const lock = {
      name: 'mystation',
      version: '0.5.0',
      packages: { '': { name: 'mystation', version: '0.5.0' }, 'node_modules/x': { version: '0.5.0' } },
    }
    const out = JSON.parse(bumpPackageLock(JSON.stringify(lock, null, 2) + '\n', '0.5.1'))
    expect(out.version).toBe('0.5.1')
    expect(out.packages[''].version).toBe('0.5.1')
    expect(out.packages['node_modules/x'].version).toBe('0.5.0')
  })
})

describe('releaseChangelog', () => {
  const log = '# Changelog\n\n## Unreleased\n\n- Fix.\n\n## 0.5.0 — 2026-10-13\n\n- Old.\n'

  it('turns Unreleased into the dated version heading', () => {
    expect(releaseChangelog(log, '0.5.1', '2026-10-16')).toBe(log.replace('## Unreleased', '## 0.5.1 — 2026-10-16'))
  })

  it('fails without an Unreleased section', () => {
    expect(() => releaseChangelog('# Changelog\n\n## 0.5.0\n\n- Old.\n', '0.5.1', '2026-10-16')).toThrow()
  })

  it('fails when the Unreleased section is empty', () => {
    expect(() => releaseChangelog('## Unreleased\n\n## 0.5.0\n- Old.\n', '0.5.1', '2026-10-16')).toThrow()
  })
})
