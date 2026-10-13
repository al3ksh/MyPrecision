// Reads release notes out of CHANGELOG.md, where each release is a `## <version>` section
// (optionally `## [<version>]` and followed by a date).

const escape = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

/** The trimmed body of the section for `version`, or null when there is none or it is empty. */
export function notesFor(changelog, version) {
  const lines = changelog.replace(/\r\n/g, '\n').split('\n')
  const heading = new RegExp(`^##\\s+\\[?${escape(version)}\\]?(\\s|$)`)
  const start = lines.findIndex((l) => heading.test(l))
  if (start < 0) return null
  const rest = lines.slice(start + 1)
  const end = rest.findIndex((l) => /^##\s/.test(l))
  const body = (end < 0 ? rest : rest.slice(0, end)).join('\n').trim()
  return body || null
}
