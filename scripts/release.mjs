// Builds the signed NSIS installer and publishes it as a GitHub Release with the
// `latest.json` manifest the in-app updater reads.
//
//   node scripts/release.mjs            build + publish v<version from tauri.conf.json>
//   node scripts/release.mjs --dry-run  build + write latest.json, publish nothing
//
// The signing key stays on this machine (~/.tauri/myprecision.key by default,
// override with TAURI_SIGNING_PRIVATE_KEY).
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'

const REPO = 'al3ksh/MyPrecision'
const dryRun = process.argv.includes('--dry-run')

const conf = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'))
const { version, productName } = conf
const tag = `v${version}`

const env = { ...process.env }
env.TAURI_SIGNING_PRIVATE_KEY ??= readFileSync(join(homedir(), '.tauri', 'myprecision.key'), 'utf8')
env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ??= ''

const run = (cmd, args, opts = {}) => execFileSync(cmd, args, { stdio: 'inherit', shell: true, env, ...opts })

run('npm', ['run', 'tauri', 'build'])

const dir = 'target/release/bundle/nsis'
const installer = `${productName}_${version}_x64-setup.exe`
const path = join(dir, installer)
if (!existsSync(`${path}.sig`)) throw new Error(`missing ${path}.sig — was the build signed?`)

// GitHub turns spaces in asset names into dots; the product name has none.
const manifest = {
  version,
  notes: `MyPrecision ${version}`,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature: readFileSync(`${path}.sig`, 'utf8').trim(),
      url: `https://github.com/${REPO}/releases/download/${tag}/${installer}`,
    },
  },
}
const latest = join(dir, 'latest.json')
writeFileSync(latest, JSON.stringify(manifest, null, 2) + '\n')
console.log(`wrote ${latest}`)

if (dryRun) {
  console.log('dry run: nothing published')
} else {
  run('gh', ['release', 'create', tag, path, latest, '--repo', REPO, '--title', tag, '--generate-notes'])
}
