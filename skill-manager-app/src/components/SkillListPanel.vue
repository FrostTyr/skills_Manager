<script setup lang="ts">
import {
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  File,
  FileCode2,
  FileText,
  Folder,
  RefreshCw,
} from 'lucide-vue-next'
import type { Skill, SkillFileContent, SkillFileEntry } from '@/types/skill'
import { agentIcon, skillAgentBadges } from '@/utils/agents'
import { useSkillStore } from '@/stores/skillStore'

defineProps<{
  collapsedDirectories: Set<string>
  expandedSkills: Set<string>
  fileLoading: Set<string>
  filteredSkills: Skill[]
  issueSummary: string | null
  selectedFile: SkillFileContent | null
  skillFileCount: (skill: Skill) => number
  visibleFiles: (skill: Skill) => SkillFileEntry[]
  directoryKey: (skillId: string, path: string) => string
}>()

const emit = defineEmits<{
  selectFile: [skill: Skill, file: SkillFileEntry]
  selectSkill: [skill: Skill]
  toggleFiles: [skill: Skill]
}>()

const store = useSkillStore()
</script>

<template>
  <main class="list-panel">
    <div v-if="issueSummary" class="issue-strip">
      <AlertTriangle :size="13" />
      <span>{{ issueSummary }}</span>
    </div>

    <section class="skill-list">
      <div v-if="store.isScanning && store.skills.length === 0" class="empty-state">
        <RefreshCw :size="24" class="spinning" />
        <p>Scanning installed skills</p>
      </div>

      <div v-else-if="filteredSkills.length === 0" class="empty-state">
        <FileText :size="28" />
        <p>{{ store.hasActiveFilter ? 'No skills match the current filters' : 'No skills found' }}</p>
      </div>

      <article
        v-for="skill in filteredSkills"
        v-else
        :key="skill.id"
        class="skill-card"
        :class="{ active: skill.id === store.selectedSkillId }"
        @click="emit('selectSkill', skill)"
      >
        <div class="skill-card-top">
          <span class="agent-label">
            <span
              v-for="agent in skillAgentBadges(
                skill.sourceAgents,
                skill.sourceAgentLabels,
                store.selectedAgents,
              )"
              :key="agent.key"
              class="agent-source"
            >
              <img class="agent-mark" :src="agentIcon(agent.key)" alt="" />
              {{ agent.label }}
            </span>
          </span>
          <span
            v-if="skill.warnings.length"
            class="version-state warning"
            :title="skill.warnings.join(', ')"
          >
            <AlertTriangle :size="12" />
            {{ skill.version === 'Unknown' ? '' : `v${skill.version}` }}
          </span>
          <span v-else class="version-state success">Local</span>
        </div>

        <h2>{{ skill.name }}</h2>
        <p>{{ skill.description }}</p>

        <div class="skill-tags">
          <span v-if="skill.category">{{ skill.category }}</span>
          <span v-for="tag in skill.customTags.slice(0, 2)" :key="tag">{{ tag }}</span>
        </div>

        <button class="file-toggle" @click.stop="emit('toggleFiles', skill)">
          <ChevronDown v-if="expandedSkills.has(skill.id)" :size="13" />
          <ChevronRight v-else :size="13" />
          <Folder :size="13" />
          <span v-if="fileLoading.has(skill.id)">Loading files</span>
          <span v-else-if="expandedSkills.has(skill.id)">Hide files</span>
          <span v-else>
            {{ skillFileCount(skill) || 'Show' }} {{ skillFileCount(skill) === 1 ? 'file' : 'files' }}
          </span>
        </button>

        <div v-if="expandedSkills.has(skill.id)" class="file-tree" @click.stop>
          <button
            v-for="file in visibleFiles(skill)"
            :key="file.relativePath"
            class="file-row"
            :class="{
              selected:
                skill.id === store.selectedSkillId &&
                selectedFile?.relativePath === file.relativePath,
            }"
            :style="{ paddingLeft: `${10 + file.depth * 14}px` }"
            @click="emit('selectFile', skill, file)"
          >
            <template v-if="file.isDirectory">
              <ChevronRight
                v-if="collapsedDirectories.has(directoryKey(skill.id, file.relativePath))"
                :size="11"
              />
              <ChevronDown v-else :size="11" />
              <Folder :size="12" />
            </template>
            <template v-else>
              <span class="file-indent"></span>
              <FileCode2 v-if="/\.(ts|tsx|js|jsx|rs|py|json|ya?ml|toml)$/i.test(file.name)" :size="12" />
              <File v-else :size="12" />
            </template>
            <span>{{ file.name }}</span>
          </button>
        </div>
      </article>
    </section>
  </main>
</template>
