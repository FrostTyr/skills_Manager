import { defineStore } from 'pinia'
import type { AgentDir, DetailViewMode, ScanIssue, Skill, SortMode } from '@/types/skill'
import { allTagsForSkill } from '@/utils/status'
import { scanSkills } from '@/utils/tauri'

interface TagCount {
  name: string
  count: number
}

interface SkillState {
  skills: Skill[]
  agents: AgentDir[]
  issues: ScanIssue[]
  availableTags: TagCount[]
  isScanning: boolean
  scanDurationMs: number
  lastScannedAt: Date | null
  searchQuery: string
  selectedAgents: string[]
  selectedTags: string[]
  selectedSkillId: string | null
  detailViewMode: DetailViewMode
  sortMode: SortMode
  errorMessage: string | null
}

export const useSkillStore = defineStore('skills', {
  state: (): SkillState => ({
    skills: [],
    agents: [],
    issues: [],
    availableTags: [],
    isScanning: false,
    scanDurationMs: 0,
    lastScannedAt: null,
    searchQuery: '',
    selectedAgents: [],
    selectedTags: [],
    selectedSkillId: null,
    detailViewMode: 'preview',
    sortMode: 'name',
    errorMessage: null,
  }),

  getters: {
    filteredSkills(state): Skill[] {
      const query = state.searchQuery.trim().toLowerCase()

      const filtered = state.skills.filter((skill) => {
        if (
          state.selectedAgents.length > 0 &&
          !state.selectedAgents.some((a) => skill.sourceAgents.includes(a))
        ) {
          return false
        }

        if (state.selectedTags.length > 0) {
          const skillTags = allTagsForSkill(skill)
          if (!state.selectedTags.every((tag) => skillTags.includes(tag))) return false
        }

        if (query) {
          const haystack = [
            skill.name,
            skill.description,
            skill.version,
            skill.author,
            skill.category,
            ...skill.sourceAgentLabels,
            ...skill.customTags,
            skill.body,
          ]
            .filter(Boolean)
            .join(' ')
            .toLowerCase()

          if (!haystack.includes(query)) return false
        }

        return true
      })

      return [...filtered].sort((a, b) => {
        if (state.sortMode === 'agent') {
          const aAgent = a.sourceAgents[0] ?? ''
          const bAgent = b.sourceAgents[0] ?? ''
          return aAgent.localeCompare(bAgent) || a.name.localeCompare(b.name)
        }

        if (state.sortMode === 'version') {
          return a.version.localeCompare(b.version) || a.name.localeCompare(b.name)
        }

        const aAgent = a.sourceAgents[0] ?? ''
        const bAgent = b.sourceAgents[0] ?? ''
        return a.name.localeCompare(b.name) || aAgent.localeCompare(bAgent)
      })
    },

    selectedSkill(): Skill | null {
      return this.filteredSkills.find((skill) => skill.id === this.selectedSkillId) ?? null
    },

    warningCount(state): number {
      return state.skills.filter((skill) => skill.warnings.length > 0 || skill.isBrokenLink).length
    },

    hasActiveFilter(state): boolean {
      return (
        state.searchQuery.trim().length > 0 ||
        state.selectedAgents.length > 0 ||
        state.selectedTags.length > 0
      )
    },
  },

  actions: {
    async refresh() {
      this.isScanning = true
      this.errorMessage = null

      try {
        const result = await scanSkills()
        this.skills = result.skills
        this.agents = result.agents
        this.issues = result.issues
        this.availableTags = buildTagCounts(result.skills)
        this.scanDurationMs = result.durationMs
        this.lastScannedAt = new Date()
        this.ensureSelection()
      } catch (error) {
        this.errorMessage = error instanceof Error ? error.message : String(error)
      } finally {
        this.isScanning = false
      }
    },

    ensureSelection() {
      const current = this.filteredSkills.find((skill) => skill.id === this.selectedSkillId)
      if (current) return

      this.selectedSkillId = this.filteredSkills[0]?.id ?? null
    },

    clearFilters() {
      this.searchQuery = ''
      this.selectedAgents = []
      this.selectedTags = []
      this.ensureSelection()
    },

    toggleAgent(agentKey: string) {
      this.selectedAgents = this.selectedAgents.includes(agentKey)
        ? this.selectedAgents.filter((key) => key !== agentKey)
        : [...this.selectedAgents, agentKey]
      this.ensureSelection()
    },

    toggleTag(tag: string, append = false) {
      if (append) {
        this.selectedTags = this.selectedTags.includes(tag)
          ? this.selectedTags.filter((selected) => selected !== tag)
          : [...this.selectedTags, tag]
      } else {
        this.selectedTags = this.selectedTags.includes(tag) ? [] : [tag]
      }

      this.ensureSelection()
    },
  },
})

function buildTagCounts(skills: Skill[]): TagCount[] {
  const counts = new Map<string, number>()

  for (const skill of skills) {
    for (const tag of allTagsForSkill(skill)) {
      counts.set(tag, (counts.get(tag) ?? 0) + 1)
    }
  }

  return [...counts.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name))
}
