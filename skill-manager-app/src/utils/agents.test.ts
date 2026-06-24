import { describe, expect, it } from 'vitest'
import { skillAgentBadges } from './agents'

describe('skillAgentBadges', () => {
  it('keeps every source agent for shared skills', () => {
    expect(skillAgentBadges(['hermes', 'codex'], ['Hermes', 'Codex'])).toEqual([
      { key: 'hermes', label: 'Hermes' },
      { key: 'codex', label: 'Codex' },
    ])
  })

  it('prioritizes the active agent filter', () => {
    expect(
      skillAgentBadges(['hermes', 'codex'], ['Hermes', 'Codex'], ['codex']),
    ).toEqual([
      { key: 'codex', label: 'Codex' },
      { key: 'hermes', label: 'Hermes' },
    ])
  })
})
