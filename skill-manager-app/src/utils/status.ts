import type { Skill } from '@/types/skill'

export const agentColors: Record<string, string> = {
  hermes: 'var(--agent-hermes)',
  codex: 'var(--agent-codex)',
  claude: 'var(--agent-claude)',
  openclaw: 'var(--agent-openclaw)',
}

export function getAgentColor(agent: string): string {
  return agentColors[agent] ?? 'var(--text-secondary)'
}

export function getSkillStatus(skill: Skill): {
  label: string
  tone: 'danger' | 'warning' | 'info' | 'success'
} {
  if (skill.isBrokenLink) return { label: 'Broken', tone: 'danger' }
  if (skill.warnings.length > 0) return { label: 'Warning', tone: 'warning' }
  if (skill.isSymlink) return { label: 'Symlink', tone: 'info' }
  return { label: 'Local', tone: 'success' }
}

export function allTagsForSkill(skill: Skill): string[] {
  return [
    ...skill.sourceAgents,
    skill.category,
    ...skill.customTags,
    skill.isSymlink ? 'symlink' : 'local',
    skill.isBrokenLink ? 'broken' : null,
  ].filter((tag): tag is string => Boolean(tag))
}
