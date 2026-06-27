<script setup lang="ts">
import {
  AlertTriangle,
  Braces,
  ChevronDown,
  ChevronLeft,
  FolderOpen,
  RefreshCw,
  X,
} from 'lucide-vue-next'
import type { AppOption, Skill, SkillFileContent } from '@/types/skill'
import { agentIcon, skillAgentBadges } from '@/utils/agents'
import { appIcon } from '@/utils/apps'
import { useSkillStore } from '@/stores/skillStore'

defineProps<{
  actionError: string | null
  actionNotice: string | null
  appMenuOpen: boolean
  availableApps: AppOption[]
  highlightedContent: string
  isSkillIndexSelected: boolean
  renderedContent: string
  selectedApp: AppOption | null
  selectedAppKey: string
  selectedFile: SkillFileContent | null
  selectedFileLoading: boolean
  selectedFileName: string
  selectedSkill: Skill | null
  showSource: boolean
}>()

const emit = defineEmits<{
  chooseApp: [app: AppOption]
  clearMessage: []
  reveal: []
  toggleAppMenu: []
}>()

const store = useSkillStore()
</script>

<template>
  <section class="detail-panel">
    <div v-if="!selectedSkill" class="empty-state detail-empty">
      <p>Select a skill to view details</p>
    </div>

    <template v-else>
      <header class="detail-toolbar">
        <div class="breadcrumb">
          <ChevronLeft :size="14" />
          <span>{{ selectedSkill.name }}</span>
          <template v-if="selectedFile && !isSkillIndexSelected">
            <span class="crumb-separator">/</span>
            <strong>{{ selectedFileName }}</strong>
          </template>
        </div>

        <div class="detail-actions">
          <button
            class="toolbar-button icon-only"
            :class="{ active: store.detailViewMode === 'source' }"
            :title="store.detailViewMode === 'preview' ? 'Source mode' : 'Preview mode'"
            @click="store.detailViewMode = store.detailViewMode === 'preview' ? 'source' : 'preview'"
          >
            <Braces :size="15" />
          </button>
          <button class="toolbar-button icon-only" title="Open folder" @click="emit('reveal')">
            <FolderOpen :size="15" />
          </button>
          <div class="app-picker" @click.stop>
            <button
              class="toolbar-button app-select"
              :disabled="availableApps.length === 0"
              @click="emit('toggleAppMenu')"
            >
              <img class="app-mark" :src="appIcon(selectedApp)" alt="" />
              <span>{{ selectedApp?.label ?? 'Open with' }}</span>
              <ChevronDown :size="12" />
            </button>
            <div v-if="appMenuOpen" class="app-menu">
              <button
                v-for="app in availableApps"
                :key="app.key"
                :class="{ selected: app.key === selectedAppKey }"
                @click="emit('chooseApp', app)"
              >
                <img class="app-mark" :src="appIcon(app)" alt="" />
                <span>{{ app.label }}</span>
              </button>
            </div>
          </div>
        </div>
      </header>

      <div v-if="actionError || actionNotice" class="action-strip" :class="{ error: actionError }">
        {{ actionError ?? actionNotice }}
        <button @click="emit('clearMessage')"><X :size="12" /></button>
      </div>

      <article class="detail-scroll">
        <header class="detail-heading">
          <h1>{{ isSkillIndexSelected ? selectedSkill.name : selectedFileName }}</h1>
          <div class="detail-tags">
            <span
              v-for="agent in skillAgentBadges(
                selectedSkill.sourceAgents,
                selectedSkill.sourceAgentLabels,
                store.selectedAgents,
              )"
              :key="agent.key"
              class="agent-chip"
            >
              <img
                class="agent-mark"
                :src="agentIcon(agent.key)"
                alt=""
              />
              {{ agent.label }}
            </span>
            <span v-if="selectedSkill.category">{{ selectedSkill.category }}</span>
            <span v-for="tag in selectedSkill.customTags" :key="tag">{{ tag }}</span>
          </div>
        </header>

        <div v-if="isSkillIndexSelected && selectedSkill.warnings.length" class="warning-box">
          <div v-for="warning in selectedSkill.warnings" :key="warning">
            <AlertTriangle :size="14" />
            <span>{{ warning }}</span>
            <X :size="12" class="warning-close" />
          </div>
        </div>

        <div class="meta-bar">
          <span><b>Version</b>{{ selectedSkill.version }}</span>
          <i></i>
          <span><b>Type</b>{{ selectedSkill.isSymlink ? 'Symlink' : 'Local' }}</span>
          <i></i>
          <span v-if="selectedFile"><b>Size</b>{{ Math.max(1, Math.round(selectedFile.size / 1024)) }} KB</span>
          <span v-else><b>Updated</b>Recently</span>
        </div>

        <div v-if="selectedFileLoading" class="content-loading">
          <RefreshCw :size="20" class="spinning" />
          Loading preview
        </div>
        <div v-else class="detail-content">
          <div v-if="!showSource" class="markdown-body" v-html="renderedContent"></div>
          <pre v-else class="source-view"><code v-html="highlightedContent"></code></pre>
        </div>
      </article>
    </template>
  </section>
</template>
