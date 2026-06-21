export interface Skill {
  id: string
  name: string
  path: string
  realPath: string
  isSymlink: boolean
  isBrokenLink: boolean
  description: string
  version: string
  author?: string | null
  category?: string | null
  customTags: string[]
  requiresAgent?: string | null
  sourceAgents: string[]
  sourceAgentLabels: string[]
  body: string
  warnings: string[]
}

export interface AgentDir {
  key: string
  label: string
  path: string
  exists: boolean
  skillCount: number
}

export interface ScanIssue {
  path: string
  level: 'warning' | 'error'
  message: string
}

export interface ScanResult {
  skills: Skill[]
  agents: AgentDir[]
  issues: ScanIssue[]
  durationMs: number
}

export interface SkillFileEntry {
  relativePath: string
  name: string
  isDirectory: boolean
  depth: number
}

export interface SkillFileContent {
  relativePath: string
  content: string
  language: string
  isMarkdown: boolean
  size: number
}

export interface AppOption {
  key: string
  label: string
}

export type DetailViewMode = 'preview' | 'source'
export type SortMode = 'name' | 'agent' | 'version'
