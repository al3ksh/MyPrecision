// Builds the signed NSIS installer and publishes it as a GitHub Release with the
// `latest.json` manifest the in-app updater reads.
//
//   node scripts/release.mjs            build + publish v<version from tauri.conf.json>
//   node scripts/release.mjs --dry-run  build + write latest.json, publish nothing
//
// The notes come from the version's section in CHANGELOG.md. Publishing needs a clean tree on
// master, level with origin/master; the script tags that commit itself and pushes the tag, so
// the release always points at what was built.
//
// The signing key stays on this machine (~/.tauri/myprecision.key by default,
// override with TAURI_SIGNING_PRIVATE_KEY).
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { notesFor } from './changelog.mjs'

const REPO = 'al3ksh/MyStation'
const dryRun = process.argv.includes('--dry-run')

const conf = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'))
const { version, productName } = conf
const tag = `v${version}`

const fail = (msg) => {
  console.error(`release: ${msg}`)
  process.exit(1)
}
const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim()
const gitOk = (...args) => {
  try {
    return git(...args)
  } catch {
    return null
  }
}

const notes = notesFor(readFileSync('CHANGELOG.md', 'utf8'), version)
if (!notes) fail(`CHANGELOG.md has no "## ${version}" section with notes`)

if (!dryRun) {
  if (git('status', '--porcelain')) fail('the working tree has uncommitted changes')
  if (git('branch', '--show-current') !== 'master') fail('releases are cut from master')
  git('fetch', '--quiet', '--tags', 'origin')
  const head = git('rev-parse', 'HEAD')
  if (head !== git('rev-parse', 'origin/master')) fail('master is not level with origin/master')
  const tagged = gitOk('rev-parse', '--verify', '--quiet', `${tag}^{commit}`)
  if (tagged && tagged !== head) fail(`${tag} already exists on another commit`)
}

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
  notes,
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
const notesFile = join(dir, 'notes.md')
writeFileSync(notesFile, notes + '\n')
console.log(`wrote ${latest}`)

if (dryRun) {
  console.log(`dry run: nothing published. Notes:\n\n${notes}`)
} else {
  if (!gitOk('rev-parse', '--verify', '--quiet', `refs/tags/${tag}`)) {
    git('tag', '-a', tag, '-m', `${productName} ${version}`)
  }
  git('push', 'origin', `refs/tags/${tag}`)
  run('gh', ['release', 'create', tag, path, latest, '--repo', REPO, '--title', tag, '--verify-tag', '--notes-file', notesFile])
}
