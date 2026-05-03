<script setup lang="ts">
import {
  AlertTriangle,
  ArrowUpDown,
  Braces,
  Clipboard,
  Eye,
  FileText,
  FolderOpen,
  RefreshCw,
  Search,
  TerminalSquare,
  X,
} from 'lucide-vue-next'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useSkillStore } from '@/stores/skillStore'
import type { Skill } from '@/types/skill'
import { renderMarkdown, renderMarkdownSource } from '@/utils/markdown'
import { openInEditor, revealInFinder } from '@/utils/tauri'
import { getAgentColor, getSkillStatus } from '@/utils/status'

const store = useSkillStore()
const searchInput = ref<HTMLInputElement | null>(null)
const actionError = ref<string | null>(null)
const actionNotice = ref<string | null>(null)
const searchDebounceTimer = ref<ReturnType<typeof setTimeout> | null>(null)

const onSearchInput = (event: Event) => {
  const value = (event.target as HTMLInputElement).value
  if (searchDebounceTimer.value !== null) clearTimeout(searchDebounceTimer.value)
  searchDebounceTimer.value = setTimeout(() => {
    store.searchQuery = value
  }, 200)
}

const filteredSkills = computed(() => store.filteredSkills)
const selectedSkill = computed(() => store.selectedSkill)
const renderedMarkdown = computed(() => renderMarkdown(selectedSkill.value?.body ?? ''))
const highlightedSource = computed(() => renderMarkdownSource(selectedSkill.value?.body ?? ''))

const issueSummary = computed(() => {
  if (store.errorMessage) return store.errorMessage
  if (store.issues.length === 0) return null
  return `${store.issues.length} scan issue${store.issues.length > 1 ? 's' : ''}`
})

const selectSkill = (skill: Skill) => {
  store.selectedSkillId = skill.id
}

const clearSearch = () => {
  if (searchDebounceTimer.value !== null) clearTimeout(searchDebounceTimer.value)
  store.searchQuery = ''
  if (searchInput.value) searchInput.value.value = ''
  store.ensureSelection()
}

const runAction = async (fn: () => Promise<void>, success: string) => {
  actionError.value = null
  actionNotice.value = null

  try {
    await fn()
    actionNotice.value = success
  } catch (error) {
    actionError.value = error instanceof Error ? error.message : String(error)
  }
}

const revealSelected = async () => {
  if (!selectedSkill.value) return
  await runAction(() => revealInFinder(selectedSkill.value!.path), 'Opened in Finder')
}

const openSelectedInCursor = async () => {
  if (!selectedSkill.value) return
  await runAction(() => openInEditor(selectedSkill.value!.path, 'cursor'), 'Opened in Cursor')
}

const copySelectedPath = async () => {
  if (!selectedSkill.value) return
  await runAction(async () => {
    await navigator.clipboard.writeText(selectedSkill.value!.path)
  }, 'Path copied')
}

const moveSelection = (offset: number) => {
  if (filteredSkills.value.length === 0) return

  const currentIndex = filteredSkills.value.findIndex((skill) => skill.id === store.selectedSkillId)
  const nextIndex =
    currentIndex === -1
      ? 0
      : Math.min(Math.max(currentIndex + offset, 0), filteredSkills.value.length - 1)
  store.selectedSkillId = filteredSkills.value[nextIndex]?.id ?? null
}

const onKeydown = (event: KeyboardEvent) => {
  if (event.metaKey && event.key.toLowerCase() === 'f') {
    event.preventDefault()
    searchInput.value?.focus()
    return
  }

  if (event.metaKey && event.key.toLowerCase() === 'r') {
    event.preventDefault()
    void store.refresh()
    return
  }

  if (event.key === 'Escape') {
    if (searchDebounceTimer.value !== null) clearTimeout(searchDebounceTimer.value)
    if (searchInput.value) searchInput.value.value = ''
    store.clearFilters()
    return
  }

  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) {
    return
  }

  if (event.key === 'ArrowDown') {
    event.preventDefault()
    moveSelection(1)
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    moveSelection(-1)
  }
}

watch(
  () => [
    store.searchQuery,
    store.selectedAgents.join(','),
    store.selectedTags.join(','),
    store.sortMode,
    store.skills.length,
  ],
  () => store.ensureSelection(),
)

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  void store.refresh()
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  if (searchDebounceTimer.value !== null) clearTimeout(searchDebounceTimer.value)
})
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <header class="brand">
        <div class="app-logo" aria-hidden="true">
          <img src="/icon.svg" alt="Skill Manager" width="26" height="26" />
        </div>
        <div>
          <h1>Skill Manager</h1>
          <span>v0.1</span>
        </div>
        <button
          class="icon-button"
          :disabled="store.isScanning"
          title="Refresh scan"
          @click="store.refresh"
        >
          <RefreshCw :size="15" :class="{ spinning: store.isScanning }" />
        </button>
      </header>

      <div class="sidebar-scroll">
        <div class="search-field">
          <Search :size="15" />
          <input
            ref="searchInput"
            :value="store.searchQuery"
            type="search"
            placeholder="Search skills"
            spellcheck="false"
            @input="onSearchInput"
          />
          <button v-if="store.searchQuery" title="Clear search" @click="clearSearch">
            <X :size="14" />
          </button>
        </div>

        <section class="filter-section">
          <div class="section-label">Agents</div>
          <button
            v-for="agent in store.agents"
            :key="agent.key"
            class="agent-row"
            :class="{
              active: store.selectedAgents.includes(agent.key),
              muted: store.selectedAgents.length > 0 && !store.selectedAgents.includes(agent.key),
              missing: !agent.exists,
            }"
            :title="agent.path"
            @click="store.toggleAgent(agent.key)"
          >
            <span class="agent-dot" :style="{ background: getAgentColor(agent.key) }"></span>
            <span>{{ agent.label }}</span>
            <small>{{ agent.skillCount }}</small>
          </button>
        </section>

        <section class="filter-section tag-section">
          <div class="section-label">Tags</div>
          <div class="tag-list">
            <button
              v-for="tag in store.availableTags"
              :key="tag.name"
              :class="{ active: store.selectedTags.includes(tag.name) }"
              @click="store.toggleTag(tag.name, $event.metaKey || $event.ctrlKey)"
            >
              <span>{{ tag.name }}</span>
              <small>{{ tag.count }}</small>
            </button>
            <span v-if="store.availableTags.length === 0" class="empty-small">No tags</span>
          </div>
        </section>
      </div>

      <footer class="stats">
        <div>
          <strong>{{ filteredSkills.length }}</strong>
          <span>/ {{ store.skills.length }} skills</span>
        </div>
        <button v-if="store.hasActiveFilter" @click="store.clearFilters">Clear</button>
      </footer>
    </aside>

    <main class="list-panel">
      <header class="panel-header">
        <div>
          <span>{{ filteredSkills.length }} skill{{ filteredSkills.length === 1 ? '' : 's' }}</span>
          <small v-if="store.scanDurationMs">scanned in {{ store.scanDurationMs }}ms</small>
        </div>
        <div class="sort-control">
          <ArrowUpDown :size="11" />
          <select v-model="store.sortMode" aria-label="Sort skills">
            <option value="name">Name</option>
            <option value="agent">Agent</option>
            <option value="version">Version</option>
          </select>
        </div>
      </header>

      <div v-if="issueSummary" class="issue-strip">
        <AlertTriangle :size="14" />
        <span>{{ issueSummary }}</span>
      </div>

      <section class="skill-list">
        <div v-if="store.isScanning && store.skills.length === 0" class="empty-state">
          <RefreshCw :size="28" class="spinning" />
          <p>Scanning installed skills</p>
        </div>

        <div v-else-if="filteredSkills.length === 0" class="empty-state">
          <FileText :size="30" />
          <p>{{ store.hasActiveFilter ? 'No skills match the current filters' : 'No skills found' }}</p>
        </div>

        <button
          v-for="skill in filteredSkills"
          v-else
          :key="skill.id"
          class="skill-row"
          :class="{ active: skill.id === store.selectedSkillId }"
          @click="selectSkill(skill)"
        >
          <div class="skill-row-top">
            <div class="skill-title-group">
              <span
                v-for="(agent, i) in skill.sourceAgents"
                :key="agent"
                class="agent-pill row-agent-pill"
                :style="{ color: getAgentColor(agent) }"
              >{{ skill.sourceAgentLabels[i] }}</span>
              <strong class="skill-name">{{ skill.name }}</strong>
            </div>
            <code class="skill-version">v{{ skill.version }}</code>
          </div>
          <p>{{ skill.description }}</p>
          <div class="skill-tags">
            <span v-if="skill.category">{{ skill.category }}</span>
            <span v-for="tag in skill.customTags.slice(0, 3)" :key="tag">{{ tag }}</span>
            <span :class="['status-pill', getSkillStatus(skill).tone]">
              {{ getSkillStatus(skill).label }}
            </span>
          </div>
        </button>
      </section>
    </main>

    <section class="detail-panel">
      <div v-if="!selectedSkill" class="empty-state detail-empty">
        <FileText :size="34" />
        <p>Select a skill to inspect its content</p>
      </div>

      <template v-else>
        <header class="detail-header">
          <div>
            <h2>{{ selectedSkill.name }}</h2>
            <div class="detail-tags">
              <span
                v-for="(agent, i) in selectedSkill.sourceAgents"
                :key="agent"
                class="agent-pill"
                :style="{ color: getAgentColor(agent) }"
              >
                <i :style="{ background: getAgentColor(agent) }"></i>
                {{ selectedSkill.sourceAgentLabels[i] }}
              </span>
              <span v-if="selectedSkill.category">{{ selectedSkill.category }}</span>
              <span v-for="tag in selectedSkill.customTags" :key="tag">{{ tag }}</span>
            </div>
          </div>

          <div class="detail-actions">
            <button
              class="icon-button"
              :title="store.detailViewMode === 'preview' ? 'Show source' : 'Show preview'"
              @click="store.detailViewMode = store.detailViewMode === 'preview' ? 'source' : 'preview'"
            >
              <Braces v-if="store.detailViewMode === 'preview'" :size="15" />
              <Eye v-else :size="15" />
            </button>
            <button class="icon-button" title="Reveal in Finder" @click="revealSelected">
              <FolderOpen :size="15" />
            </button>
            <button class="icon-button" title="Open in Cursor" @click="openSelectedInCursor">
              <TerminalSquare :size="15" />
            </button>
            <button class="icon-button" title="Copy path" @click="copySelectedPath">
              <Clipboard :size="15" />
            </button>
          </div>
        </header>

        <div class="meta-bar">
          <span><b>Version</b>{{ selectedSkill.version }}</span>
          <span v-if="selectedSkill.author"><b>Author</b>{{ selectedSkill.author }}</span>
          <span><b>Type</b>{{ selectedSkill.isSymlink ? 'Symlink' : 'Local' }}</span>
        </div>

        <div v-if="selectedSkill.warnings.length > 0" class="warning-box">
          <div v-for="warning in selectedSkill.warnings" :key="warning">
            <AlertTriangle :size="14" />
            <span>{{ warning }}</span>
          </div>
        </div>

        <div v-if="actionError || actionNotice" class="action-strip" :class="{ error: actionError }">
          {{ actionError ?? actionNotice }}
        </div>

        <article class="detail-content">
          <div
            v-if="store.detailViewMode === 'preview'"
            class="markdown-body"
            v-html="renderedMarkdown"
          ></div>
          <pre v-else class="source-view"><code v-html="highlightedSource"></code></pre>
        </article>
      </template>
    </section>
  </div>
</template>

<style scoped>
.app-shell {
  display: grid;
  grid-template-columns: 260px minmax(320px, 440px) minmax(420px, 1fr);
  height: 100vh;
  background: var(--bg);
  color: var(--text);
}

.sidebar,
.list-panel,
.detail-panel {
  min-height: 0;
}

.sidebar {
  display: flex;
  flex-direction: column;
  background: var(--sidebar-bg);
  border-right: 1px solid var(--border);
}

.brand,
.panel-header,
.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.brand {
  min-height: 68px;
  padding: 16px;
  border-bottom: 1px solid var(--divider);
  justify-content: flex-start;
  gap: 10px;
}

.brand > div:nth-child(2) {
  flex: 1;
  min-width: 0;
}

.app-logo {
  flex: 0 0 auto;
}

.app-logo img {
  display: block;
  width: 32px;
  height: 32px;
}

.brand h1 {
  margin: 0;
  font-size: 16px;
  line-height: 1.2;
}

.brand span,
.panel-header small {
  display: block;
  margin-top: 3px;
  color: var(--text-muted);
  font-family: var(--mono);
  font-size: 10px;
}

.sidebar-scroll {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  padding: 16px 14px;
}

.search-field {
  position: relative;
  margin-bottom: 22px;
}

.search-field svg {
  position: absolute;
  left: 10px;
  top: 50%;
  color: var(--text-muted);
  transform: translateY(-50%);
}

.search-field input {
  width: 100%;
  height: 34px;
  padding: 0 34px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--text);
  outline: none;
}

.search-field input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(0, 0, 0, 0.04);
}

.search-field button {
  position: absolute;
  right: 4px;
  top: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: 0;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transform: translateY(-50%);
}

.filter-section {
  flex: 0 0 auto;
  margin-bottom: 22px;
}

.tag-section {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  margin-bottom: 0;
}

.section-label {
  margin: 0 0 10px 4px;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.agent-row {
  display: flex;
  align-items: center;
  width: 100%;
  margin-bottom: 2px;
  padding: 7px 9px;
  border: 0;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text);
  cursor: pointer;
  text-align: left;
}

.agent-row:hover {
  background: var(--accent-muted);
}

.agent-row.active {
  background: var(--highlight-muted);
  color: var(--highlight);
}

.agent-row.muted,
.agent-row.missing {
  color: var(--text-disabled);
}

.agent-dot {
  width: 7px;
  height: 7px;
  margin-right: 8px;
  border-radius: 50%;
  flex: 0 0 auto;
}

.agent-row span:nth-child(2) {
  flex: 1;
}

.agent-row small,
.tag-list small {
  color: var(--text-muted);
  font-size: 10px;
}

.tag-list {
  display: grid;
  flex: 1;
  align-content: start;
  gap: 2px;
  min-height: 0;
  overflow: auto;
  padding: 0 4px;
}

.tag-list button,
.stats button {
  display: inline-flex;
  align-items: center;
  min-height: 24px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 12px;
}

.tag-list button {
  justify-content: space-between;
  width: 100%;
  min-height: 30px;
  padding: 4px 9px;
  text-align: left;
}

.tag-list button span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tag-list button.active {
  border-color: var(--highlight);
  background: var(--highlight-muted);
  color: var(--highlight);
  font-weight: 700;
}

.empty-small {
  color: var(--text-muted);
  font-size: 12px;
}

.stats {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 16px;
  border-top: 1px solid var(--divider);
}

.stats strong {
  font-size: 24px;
  line-height: 1;
}

.stats span {
  margin-left: 4px;
  color: var(--text-muted);
  font-size: 12px;
}

.stats button {
  padding: 4px 8px;
}

.list-panel {
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border);
  background: var(--bg);
}

.panel-header {
  min-height: 56px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--divider);
}

.panel-header span {
  color: var(--text-secondary);
  font-size: 13px;
}

.sort-control {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 26px;
  padding: 0 2px 0 7px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--text-muted);
  font-size: 12px;
}

.sort-control select {
  height: 100%;
  padding: 0 2px;
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  outline: none;
  cursor: pointer;
}

.issue-strip,
.action-strip {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 34px;
  padding: 8px 16px;
  border-bottom: 1px solid rgba(217, 119, 6, 0.14);
  background: var(--warning-bg);
  color: var(--warning);
  font-size: 12px;
}

.action-strip {
  border-bottom-color: var(--divider);
  background: var(--success-bg);
  color: var(--success);
}

.action-strip.error {
  background: var(--danger-bg);
  color: var(--danger);
}

.skill-list,
.detail-content {
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.skill-row {
  display: block;
  width: 100%;
  padding: 13px 16px 12px;
  border: 0;
  border-bottom: 1px solid var(--divider);
  border-left: 2px solid transparent;
  background: var(--bg);
  cursor: pointer;
  text-align: left;
}

.skill-row:hover,
.skill-row.active {
  background: rgba(0, 0, 0, 0.018);
}

.skill-row.active {
  border-left-color: var(--highlight);
  background: var(--highlight-muted);
}

.skill-row-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 5px;
}

.skill-title-group {
  display: flex;
  align-items: center;
  flex: 1 1 auto;
  gap: 5px;
  min-width: 0;
}

.skill-name {
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
  font-size: 14px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-agent-pill {
  display: inline-flex;
  align-items: center;
  flex: 0 0 auto;
  gap: 4px;
  max-width: 96px;
  min-height: 19px;
  padding: 1px 7px;
  overflow: hidden;
  border-radius: 3px;
  background: var(--surface-muted);
  font-size: 10px;
  font-weight: 500;
  white-space: nowrap;
}

.skill-version {
  flex: 0 0 auto;
  color: var(--text-muted);
  font-size: 11px;
}

.skill-row p {
  overflow: hidden;
  margin: 0 0 8px;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.45;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.skill-tags,
.detail-tags {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  flex-wrap: wrap;
}

.skill-tags span,
.detail-tags span {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-height: 19px;
  padding: 1px 7px;
  border-radius: 3px;
  background: var(--surface-muted);
  color: var(--text-secondary);
  font-size: 10px;
  font-weight: 500;
}

.agent-pill i {
  width: 5px;
  height: 5px;
  border-radius: 50%;
}

.skill-tags .status-pill {
  margin-left: auto;
  background: transparent;
}

.status-pill.danger {
  color: var(--danger);
}

.status-pill.warning {
  color: var(--warning);
}

.status-pill.info {
  color: var(--info);
}

.status-pill.success {
  color: var(--success);
}

.detail-panel {
  display: flex;
  min-width: 0;
  flex-direction: column;
  background: var(--bg);
}

.detail-header {
  gap: 16px;
  min-height: 94px;
  padding: 22px 24px 16px;
  border-bottom: 1px solid var(--divider);
  align-items: flex-start;
}

.detail-header h2 {
  margin: 0 0 8px;
  font-size: 20px;
  line-height: 1.2;
}

.detail-tags span {
  font-size: 11px;
}

.detail-actions {
  display: flex;
  gap: 7px;
  flex: 0 0 auto;
}

.meta-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 24px;
  padding: 12px 24px;
  border-bottom: 1px solid var(--divider);
  color: var(--text);
  font-size: 12px;
}

.meta-bar b {
  margin-right: 6px;
  color: var(--text-muted);
  font-weight: 500;
}

.warning-box {
  margin: 14px 24px 0;
  padding: 10px 12px;
  border: 1px solid rgba(217, 119, 6, 0.16);
  border-radius: var(--radius);
  background: var(--warning-bg);
  color: var(--warning);
  font-size: 12px;
}

.warning-box div {
  display: flex;
  align-items: center;
  gap: 7px;
}

.detail-content {
  padding: 24px;
}

.source-view {
  min-height: 100%;
  margin: 0;
  padding: 16px;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
  color: var(--text);
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
}

.source-view code {
  display: block;
  min-width: 100%;
  background: transparent;
}

.empty-state {
  display: flex;
  height: 100%;
  min-height: 240px;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 12px;
  padding: 32px;
  color: var(--text-disabled);
  text-align: center;
}

.empty-state p {
  margin: 0;
  color: var(--text-secondary);
  font-size: 13px;
}

.detail-empty {
  min-height: 100%;
}

.spinning {
  animation: spin 0.75s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

:deep(.markdown-body) {
  color: var(--text);
  font-size: 14px;
  line-height: 1.7;
}

:deep(.markdown-body h1),
:deep(.markdown-body h2),
:deep(.markdown-body h3) {
  margin: 24px 0 8px;
  line-height: 1.25;
}

:deep(.markdown-body h1:first-child),
:deep(.markdown-body h2:first-child),
:deep(.markdown-body h3:first-child) {
  margin-top: 0;
}

:deep(.markdown-body h1) {
  font-size: 22px;
}

:deep(.markdown-body h2) {
  font-size: 18px;
}

:deep(.markdown-body h3) {
  font-size: 15px;
}

:deep(.markdown-body p) {
  margin: 7px 0;
}

:deep(.markdown-body code) {
  padding: 1px 5px;
  border-radius: 3px;
  background: var(--surface-muted);
  font-size: 12px;
}

:deep(.markdown-body pre) {
  overflow: auto;
  padding: 14px 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
  font-size: 13px;
  line-height: 1.55;
}

:deep(.markdown-body pre code) {
  padding: 0;
  background: transparent;
}

:deep(.markdown-body table) {
  width: 100%;
  border-collapse: collapse;
  margin: 12px 0;
}

:deep(.markdown-body th),
:deep(.markdown-body td) {
  padding: 7px 10px;
  border-bottom: 1px solid var(--divider);
  text-align: left;
}

:deep(.markdown-body blockquote) {
  margin: 12px 0;
  padding-left: 14px;
  border-left: 2px solid var(--border);
  color: var(--text-secondary);
}
</style>
