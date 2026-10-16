// Sets the app version everywhere it lives and dates the changelog's Unreleased section.
//
//   node scripts/bump.mjs 0.5.1
import { readFileSync, writeFileSync } from 'node:fs'
import { bumpCargoLock, bumpCargoToml, bumpJson, bumpPackageLock, releaseChangelog } from './version.mjs'

const version = process.argv[2]
if (!/^\d+\.\d+\.\d+$/.test(version ?? '')) {
  console.error('usage: npm run bump -- <major.minor.patch>')
  process.exit(1)
}
const today = new Date().toLocaleDateString('sv-SE')

const edits = {
  'package.json': (t) => bumpJson(t, version),
  'package-lock.json': (t) => bumpPackageLock(t, version),
  'src-tauri/tauri.conf.json': (t) => bumpJson(t, version),
  'crates/core/Cargo.toml': (t) => bumpCargoToml(t, version),
  'src-tauri/Cargo.toml': (t) => bumpCargoToml(t, version),
  'Cargo.lock': (t) => bumpCargoLock(t, ['mystation', 'mystation-core'], version),
  'CHANGELOG.md': (t) => releaseChangelog(t, version, today),
}

// Compute every change before writing any, so a failure leaves the tree untouched.
const out = Object.entries(edits).map(([file, edit]) => {
  const text = readFileSync(file, 'utf8')
  const crlf = text.includes('\r\n')
  const changed = edit(text.replace(/\r\n/g, '\n'))
  return [file, crlf ? changed.replace(/\n/g, '\r\n') : changed]
})
for (const [file, text] of out) writeFileSync(file, text)
console.log(`bumped to ${version}: ${out.map(([f]) => f).join(', ')}`)
