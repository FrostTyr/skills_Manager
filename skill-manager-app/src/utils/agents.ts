const KNOWN_AGENTS = new Set(['hermes', 'codex', 'claude', 'openclaw'])

export interface AgentBadge {
  key: string
  label: string
}

export function agentIcon(agent: string): string {
  return `/agents/${KNOWN_AGENTS.has(agent) ? agent : 'codex'}.png`
}

export function skillAgentBadges(
  sourceAgents: string[],
  sourceAgentLabels: string[],
  preferredAgents: string[] = [],
): AgentBadge[] {
  const badges = sourceAgents.map((key, index) => ({
    key,
    label: sourceAgentLabels[index] ?? key,
  }))
  const preference = new Map(preferredAgents.map((key, index) => [key, index]))

  return badges.sort((a, b) => {
    const aRank = preference.get(a.key) ?? Number.MAX_SAFE_INTEGER
    const bRank = preference.get(b.key) ?? Number.MAX_SAFE_INTEGER
    return aRank - bRank
  })
}
