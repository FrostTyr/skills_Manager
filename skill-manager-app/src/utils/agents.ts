const AGENT_ICONS: Record<string, string> = {
  hermes: '/agents/hermes.png',
  codex: '/agents/codex.png',
  claude: '/agents/claude.svg',
  openclaw: '/agents/openclaw.svg',
}

export interface AgentBadge {
  key: string
  label: string
}

export function agentIcon(agent: string): string {
  return AGENT_ICONS[agent] ?? AGENT_ICONS.codex
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