import { describe, expect, it } from 'vitest'
import { notesFor } from './changelog.mjs'

const changelog = `# Changelog

Intro text.

## Unreleased

- Work in progress.

## 0.4.0 — 2026-10-12

- Desk and Remote battery profiles.
- Per-app graphics choice.

## 0.3.0 — 2026-10-09

- Hardware page.
`

describe('notesFor', () => {
  it('returns the body of the section for that version', () => {
    expect(notesFor(changelog, '0.4.0')).toBe('- Desk and Remote battery profiles.\n- Per-app graphics choice.')
  })

  it('reads the last section up to the end of the file', () => {
    expect(notesFor(changelog, '0.3.0')).toBe('- Hardware page.')
  })

  it('accepts a heading without a date or in brackets', () => {
    expect(notesFor('## [1.0.0]\n\nFirst.\n', '1.0.0')).toBe('First.')
    expect(notesFor('## 1.0.0\nFirst.', '1.0.0')).toBe('First.')
  })

  it('does not match a version that only starts the same', () => {
    expect(notesFor('## 0.4.10\n\n- Later.\n', '0.4.1')).toBeNull()
  })

  it('is null when the version is missing or its section is empty', () => {
    expect(notesFor(changelog, '0.5.0')).toBeNull()
    expect(notesFor('## 0.5.0\n\n## 0.4.0\n- x', '0.5.0')).toBeNull()
  })

  it('handles Windows line endings', () => {
    expect(notesFor('## 0.5.0\r\n\r\n- One.\r\n- Two.\r\n', '0.5.0')).toBe('- One.\n- Two.')
  })
})
