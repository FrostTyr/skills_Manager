import { describe, expect, it } from 'vitest'
import type { Skill } from '@/types/skill'
import { allTagsForSkill, getSkillStatus } from './status'

const baseSkill: Skill = {
  id: 'codex:/tmp/example',
  name: 'example',
  path: '/tmp/example',
  realPath: '/tmp/example',
  isSymlink: false,
  isBrokenLink: false,
  description: 'Example skill',
  version: '1.0.0',
  author: null,
  category: 'utility',
  customTags: ['automation'],
  requiresAgent: null,
  sourceAgents: ['codex'],
  sourceAgentLabels: ['Codex'],
  body: '# Example',
  warnings: [],
}

describe('status helpers', () => {
  it('builds filter tags from all source agents, category, custom tags, and storage state', () => {
    expect(allTagsForSkill(baseSkill)).toEqual(['codex', 'utility', 'automation', 'local'])
  })

  it('prioritizes broken symlink status over warnings', () => {
    expect(
      getSkillStatus({
        ...baseSkill,
        isSymlink: true,
        isBrokenLink: true,
        warnings: ['missing target'],
      }),
    ).toEqual({ label: 'Broken', tone: 'danger' })
  })
})
