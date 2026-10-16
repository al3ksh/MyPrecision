// Version bumps for every file that carries the app version. Each function takes the file's text
// and returns it changed, throwing when the field it should change is missing.

const replaceOnce = (text, pattern, replacement, what) => {
  if (!pattern.test(text)) throw new Error(`no ${what} to bump`)
  return text.replace(pattern, replacement)
}

/** `version` in the `[package]` table of a Cargo.toml. */
export function bumpCargoToml(text, version) {
  return replaceOnce(text, /(\[package\][^[]*?\nversion = ")[^"]*(")/, `$1${version}$2`, '[package] version')
}

/** The `[[package]]` entries for `names` in Cargo.lock. */
export function bumpCargoLock(text, names, version) {
  return names.reduce(
    (out, name) =>
      replaceOnce(out, new RegExp(`(\\nname = "${name}"\\nversion = ")[^"]*(")`), `$1${version}$2`, `Cargo.lock entry for ${name}`),
    text,
  )
}

/** The first `"version"` key, which in package.json and tauri.conf.json is the top-level one. */
export function bumpJson(text, version) {
  return replaceOnce(text, /("version":\s*")[^"]*(")/, `$1${version}$2`, '"version"')
}

/** The root version and the root package's version in package-lock.json. */
export function bumpPackageLock(text, version) {
  const lock = JSON.parse(text)
  if (!lock.packages?.['']) throw new Error('no root package in package-lock.json')
  lock.version = version
  lock.packages[''].version = version
  return JSON.stringify(lock, null, 2) + '\n'
}

/** Renames the `## Unreleased` section to `## <version> — <date>`. */
export function releaseChangelog(text, version, date) {
  const lines = text.split('\n')
  const start = lines.findIndex((l) => /^##\s+Unreleased\s*$/.test(l))
  if (start < 0) throw new Error('CHANGELOG.md has no "## Unreleased" section')
  const rest = lines.slice(start + 1)
  const end = rest.findIndex((l) => /^##\s/.test(l))
  if (!(end < 0 ? rest : rest.slice(0, end)).join('').trim()) throw new Error('the Unreleased section is empty')
  lines[start] = `## ${version} — ${date}`
  return lines.join('\n')
}
