<script setup lang="ts">
import {
  AlertTriangle,
  Braces,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Code2,
  File,
  FileCode2,
  FileText,
  Folder,
  FolderOpen,
  Hash,
  RefreshCw,
  Search,
  Shell,
  X,
} from 'lucide-vue-next'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useSkillStore } from '@/stores/skillStore'
import type { AppOption, Skill, SkillFileContent, SkillFileEntry } from '@/types/skill'
import { renderMarkdown, renderSource } from '@/utils/markdown'
import {
  getAvailableApps,
  listSkillFiles,
  openInApp,
  readSkillFile,
  revealInFinder,
} from '@/utils/tauri'

const store = useSkillStore()
const searchInput = ref<HTMLInputElement | null>(null)
const actionError = ref<string | null>(null)
const actionNotice = ref<string | null>(null)
const searchDebounceTimer = ref<ReturnType<typeof setTimeout> | null>(null)
const fileLists = ref<Record<string, SkillFileEntry[]>>({})
const fileLoading = ref(new Set<string>())
const expandedSkills = ref(new Set<string>())
const collapsedDirectories = ref(new Set<string>())
const selectedFile = ref<SkillFileContent | null>(null)
const selectedFileLoading = ref(false)
const availableApps = ref<AppOption[]>([])
const selectedAppKey = ref('')
const appMenuOpen = ref(false)

const filteredSkills = computed(() => store.filteredSkills)
const selectedSkill = computed(() => store.selectedSkill)
const selectedApp = computed(
  () => availableApps.value.find((app) => app.key === selectedAppKey.value) ?? null,
)
const selectedFileName = computed(() => selectedFile.value?.relativePath.split('/').pop() ?? '')
const isSkillIndexSelected = computed(
  () => !selectedFile.value || selectedFile.value.relativePath.toLowerCase() === 'skill.md',
)
const previewContent = computed(() => selectedFile.value?.content ?? selectedSkill.value?.body ?? '')
const isMarkdownFile = computed(() => selectedFile.value?.isMarkdown ?? true)
const renderedContent = computed(() => renderMarkdown(previewContent.value))
const highlightedContent = computed(() =>
  renderSource(previewContent.value, selectedFile.value?.language ?? 'markdown'),
)
const showSource = computed(
  () => store.detailViewMode === 'source' || !isMarkdownFile.value,
)

const issueSummary = computed(() => {
  if (store.errorMessage) return store.errorMessage
  if (store.issues.length === 0) return null
  return `${store.issues.length} scan issue${store.issues.length > 1 ? 's' : ''}`
})

const onSearchInput = (event: Event) => {
  const value = (event.target as HTMLInputElement).value
  if (searchDebounceTimer.value !== null) clearTimeout(searchDebounceTimer.value)
  searchDebounceTimer.value = setTimeout(() => {
    store.searchQuery = value
  }, 180)
}

const clearSearch = () => {
  if (searchDebounceTimer.value !== null) clearTimeout(searchDebounceTimer.value)
  store.searchQuery = ''
  if (searchInput.value) searchInput.value.value = ''
  store.ensureSelection()
}

const agentIcon = (agent: string) => {
  return `/agents/${['hermes', 'codex', 'claude', 'openclaw'].includes(agent) ? agent : 'codex'}.png`
}

const selectSkill = async (skill: Skill) => {
  store.selectedSkillId = skill.id
  selectedFile.value = null
  await ensureFiles(skill)
}

const ensureFiles = async (skill: Skill) => {
  if (fileLists.value[skill.id] || fileLoading.value.has(skill.id)) return

  fileLoading.value = new Set(fileLoading.value).add(skill.id)
  try {
    const files = await listSkillFiles(skill.path)
    fileLists.value = { ...fileLists.value, [skill.id]: files }
  } catch (error) {
    actionError.value = error instanceof Error ? error.message : String(error)
  } finally {
    const next = new Set(fileLoading.value)
    next.delete(skill.id)
    fileLoading.value = next
  }
}

const toggleSkillFiles = async (skill: Skill) => {
  const next = new Set(expandedSkills.value)
  if (next.has(skill.id)) {
    next.delete(skill.id)
  } else {
    next.add(skill.id)
    await ensureFiles(skill)
  }
  expandedSkills.value = next
}

const directoryKey = (skillId: string, path: string) => `${skillId}:${path}`

const toggleDirectory = (skillId: string, path: string) => {
  const key = directoryKey(skillId, path)
  const next = new Set(collapsedDirectories.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  collapsedDirectories.value = next
}

const visibleFiles = (skill: Skill) => {
  const files = fileLists.value[skill.id] ?? []
  return files.filter((file) => {
    const segments = file.relativePath.split('/')
    segments.pop()
    let ancestor = ''
    for (const segment of segments) {
      ancestor = ancestor ? `${ancestor}/${segment}` : segment
      if (collapsedDirectories.value.has(directoryKey(skill.id, ancestor))) return false
    }
    return true
  })
}

const skillFileCount = (skill: Skill) =>
  (fileLists.value[skill.id] ?? []).filter((file) => !file.isDirectory).length

const selectFile = async (skill: Skill, file: SkillFileEntry) => {
  if (file.isDirectory) {
    toggleDirectory(skill.id, file.relativePath)
    return
  }

  if (store.selectedSkillId !== skill.id) store.selectedSkillId = skill.id
  selectedFileLoading.value = true
  actionError.value = null
  try {
    selectedFile.value = await readSkillFile(skill.path, file.relativePath)
  } catch (error) {
    selectedFile.value = {
      relativePath: file.relativePath,
      content: error instanceof Error ? error.message : String(error),
      language: 'plaintext',
      isMarkdown: false,
      size: 0,
    }
  } finally {
    selectedFileLoading.value = false
  }
}

const runAction = async (fn: () => Promise<void>, success: string) => {
  actionError.value = null
  actionNotice.value = null
  try {
    await fn()
    actionNotice.value = success
    window.setTimeout(() => {
      actionNotice.value = null
    }, 2200)
  } catch (error) {
    actionError.value = error instanceof Error ? error.message : String(error)
  }
}

const revealSelected = async () => {
  if (!selectedSkill.value) return
  await runAction(() => revealInFinder(selectedSkill.value!.path), 'Opened in Finder')
}

const chooseAndOpenApp = async (app: AppOption) => {
  selectedAppKey.value = app.key
  appMenuOpen.value = false
  if (!selectedSkill.value) return
  await runAction(
    () => openInApp(selectedSkill.value!.path, app.key),
    `Opened in ${app.label}`,
  )
}

const moveSelection = (offset: number) => {
  if (filteredSkills.value.length === 0) return
  const currentIndex = filteredSkills.value.findIndex((skill) => skill.id === store.selectedSkillId)
  const nextIndex =
    currentIndex === -1
      ? 0
      : Math.min(Math.max(currentIndex + offset, 0), filteredSkills.value.length - 1)
  const skill = filteredSkills.value[nextIndex]
  if (skill) void selectSkill(skill)
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
    appMenuOpen.value = false
    if (store.hasActiveFilter) {
      clearSearch()
      store.clearFilters()
    }
    return
  }
  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) return
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    moveSelection(1)
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    moveSelection(-1)
  }
}

const closeMenus = () => {
  appMenuOpen.value = false
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

watch(
  () => store.selectedSkillId,
  async () => {
    selectedFile.value = null
    if (selectedSkill.value) {
      await ensureFiles(selectedSkill.value)
    }
  },
)

onMounted(async () => {
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('click', closeMenus)
  const [, apps] = await Promise.all([store.refresh(), getAvailableApps()])
  availableApps.value = apps
  selectedAppKey.value =
    apps.find((app) => app.key === 'cursor')?.key ??
    apps.find((app) => app.key !== 'finder')?.key ??
    apps[0]?.key ??
    ''
  if (selectedSkill.value) {
    await ensureFiles(selectedSkill.value)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('click', closeMenus)
  if (searchDebounceTimer.value !== null) clearTimeout(searchDebounceTimer.value)
})
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <header class="brand">
        <div class="app-logo" aria-hidden="true">
          <span>S</span>
        </div>
        <div class="brand-copy">
          <h1>Skill Manager</h1>
          <span>v0.1</span>
        </div>
        <button
          class="bare-icon"
          :disabled="store.isScanning"
          title="Refresh scan"
          @click="store.refresh"
        >
          <RefreshCw :size="14" :class="{ spinning: store.isScanning }" />
        </button>
        <div class="skill-counter">
          <strong>{{ filteredSkills.length }}</strong>
          <span>/ {{ store.skills.length }} skills</span>
        </div>
      </header>

      <div class="sidebar-scroll">
        <div class="search-field">
          <Search :size="14" />
          <input
            ref="searchInput"
            :value="store.searchQuery"
            type="search"
            placeholder="Search skills"
            spellcheck="false"
            @input="onSearchInput"
          />
          <button v-if="store.searchQuery" title="Clear search" @click="clearSearch">
            <X :size="13" />
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
            <img class="agent-mark" :src="agentIcon(agent.key)" alt="" />
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
              <Hash :size="12" />
              <span>{{ tag.name }}</span>
              <small>{{ tag.count }}</small>
            </button>
            <span v-if="store.availableTags.length === 0" class="empty-small">No tags</span>
          </div>
        </section>
      </div>
    </aside>

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
          @click="selectSkill(skill)"
        >
          <div class="skill-card-top">
            <span class="agent-label">
              <img class="agent-mark" :src="agentIcon(skill.sourceAgents[0] ?? '')" alt="" />
              {{ skill.sourceAgentLabels[0] }}
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

          <button class="file-toggle" @click.stop="toggleSkillFiles(skill)">
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
              @click="selectFile(skill, file)"
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
            <button class="toolbar-button icon-only" title="Open folder" @click="revealSelected">
              <FolderOpen :size="15" />
            </button>
            <div class="app-picker" @click.stop>
              <button
                class="toolbar-button app-select"
                :disabled="availableApps.length === 0"
                @click="appMenuOpen = !appMenuOpen"
              >
                <Code2 :size="14" />
                <span>{{ selectedApp?.label ?? 'Open with' }}</span>
                <ChevronDown :size="12" />
              </button>
              <div v-if="appMenuOpen" class="app-menu">
                <button
                  v-for="app in availableApps"
                  :key="app.key"
                  :class="{ selected: app.key === selectedAppKey }"
                  @click="chooseAndOpenApp(app)"
                >
                  <Code2 v-if="!['terminal', 'warp', 'ghostty', 'finder'].includes(app.key)" :size="14" />
                  <Shell v-else-if="app.key !== 'finder'" :size="14" />
                  <FolderOpen v-else :size="14" />
                  <span>{{ app.label }}</span>
                </button>
              </div>
            </div>
          </div>
        </header>

        <div v-if="actionError || actionNotice" class="action-strip" :class="{ error: actionError }">
          {{ actionError ?? actionNotice }}
          <button @click="actionError = null; actionNotice = null"><X :size="12" /></button>
        </div>

        <article class="detail-scroll">
          <header class="detail-heading">
            <h1>{{ isSkillIndexSelected ? selectedSkill.name : selectedFileName }}</h1>
            <div class="detail-tags">
              <span class="agent-chip">
                <img
                  class="agent-mark"
                  :src="agentIcon(selectedSkill.sourceAgents[0] ?? '')"
                  alt=""
                />
                {{ selectedSkill.sourceAgentLabels[0] }}
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
            <div
              v-if="!showSource"
              class="markdown-body"
              v-html="renderedContent"
            ></div>
            <pre v-else class="source-view"><code v-html="highlightedContent"></code></pre>
          </div>
        </article>
      </template>
    </section>
  </div>
</template>

<style scoped>
.app-shell {
  display: grid;
  grid-template-columns: 21% 30% 1fr;
  width: 100%;
  height: 100vh;
  background: #fff;
  color: #18181b;
}

.sidebar,
.list-panel,
.detail-panel {
  min-width: 0;
  min-height: 0;
}

.sidebar {
  display: flex;
  flex-direction: column;
  background: #f7f7f8;
  border-right: 1px solid #e9e9eb;
}

.brand {
  position: relative;
  display: grid;
  grid-template-columns: 28px 1fr 24px;
  gap: 9px;
  min-height: 84px;
  padding: 13px 14px 11px;
  border-bottom: 1px solid #ececef;
}

.app-logo {
  display: flex;
  width: 25px;
  height: 25px;
  align-items: center;
  justify-content: center;
  margin-top: 0;
  overflow: hidden;
  border-radius: 7px;
  background: #151518;
}

.app-logo span {
  color: #fff;
  font-size: 12px;
  font-weight: 750;
  line-height: 1;
}

.brand-copy h1 {
  margin: 0;
  font-size: 13px;
  font-weight: 700;
  line-height: 1.25;
}

.brand-copy > span {
  display: block;
  margin-top: 2px;
  color: #a1a1aa;
  font-size: 10px;
}

.bare-icon {
  display: inline-flex;
  width: 24px;
  height: 24px;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 0;
  background: transparent;
  color: #a1a1aa;
  cursor: pointer;
}

.bare-icon:hover {
  color: #52525b;
}

.skill-counter {
  display: inline-flex;
  width: fit-content;
  min-height: 25px;
  align-items: center;
  grid-column: 1 / -1;
  margin-top: 1px;
  padding: 3px 9px;
  border-radius: 999px;
  background: #f0f0f2;
  color: #71717a;
  font-size: 11px;
}

.skill-counter strong {
  color: #52525b;
  font-weight: 600;
}

.sidebar-scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 14px 11px;
}

.search-field {
  position: relative;
  margin-bottom: 19px;
}

.search-field > svg {
  position: absolute;
  left: 10px;
  top: 50%;
  color: #a1a1aa;
  transform: translateY(-50%);
}

.search-field input {
  width: 100%;
  height: 34px;
  padding: 0 31px;
  border: 1px solid #e4e4e7;
  border-radius: 6px;
  background: #fff;
  color: #27272a;
  font-size: 12px;
  outline: none;
}

.search-field input:focus {
  border-color: #c7c7cd;
  box-shadow: 0 0 0 2px rgba(24, 24, 27, 0.035);
}

.search-field button {
  position: absolute;
  right: 4px;
  top: 50%;
  display: flex;
  width: 23px;
  height: 23px;
  align-items: center;
  justify-content: center;
  border: 0;
  background: transparent;
  color: #a1a1aa;
  transform: translateY(-50%);
}

.filter-section {
  margin-bottom: 20px;
}

.section-label {
  margin: 0 0 7px 3px;
  color: #a1a1aa;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.agent-row,
.tag-list button {
  display: flex;
  width: 100%;
  align-items: center;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: #52525b;
  cursor: pointer;
  text-align: left;
}

.agent-row {
  height: 38px;
  gap: 9px;
  padding: 0 8px;
  font-size: 12px;
}

.agent-row:hover,
.agent-row.active,
.tag-list button:hover,
.tag-list button.active {
  background: #ededf0;
  color: #18181b;
}

.agent-row.muted,
.agent-row.missing {
  color: #b8b8bf;
}

.agent-row span,
.tag-list button span {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.agent-row > .agent-mark,
.agent-label > .agent-mark {
  width: 22px;
  height: 22px;
  object-fit: contain;
}

.detail-tags .agent-chip > .agent-mark {
  width: 18px;
  height: 18px;
  object-fit: contain;
}

.agent-row small,
.tag-list small {
  color: #a1a1aa;
  font-size: 10px;
}

.tag-list {
  display: grid;
  gap: 1px;
}

.tag-list button {
  height: 34px;
  gap: 7px;
  padding: 0 8px;
  font-size: 11px;
}

.tag-list button > svg {
  color: #a1a1aa;
}

.empty-small {
  display: block;
  padding: 8px;
  color: #a1a1aa;
  font-size: 11px;
}

.list-panel {
  display: flex;
  flex-direction: column;
  background: #f7f7f8;
  border-right: 1px solid #e8e8eb;
}

.issue-strip,
.action-strip {
  display: flex;
  min-height: 34px;
  align-items: center;
  gap: 8px;
  padding: 7px 14px;
  border-bottom: 1px solid #f1dcc0;
  background: #fff9ed;
  color: #a16207;
  font-size: 11px;
}

.skill-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 12px 11px 18px;
}

.skill-card {
  margin-bottom: 9px;
  padding: 17px 15px 14px;
  border: 1px solid #e3e3e7;
  border-radius: 9px;
  background: #fff;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(15, 23, 42, 0.025);
  transition: border-color 0.12s ease, box-shadow 0.12s ease;
}

.skill-card:hover {
  border-color: #d5d5da;
}

.skill-card.active {
  border-color: #cfcfd5;
  box-shadow: 0 1px 3px rgba(15, 23, 42, 0.09);
}

.skill-card-top {
  display: flex;
  min-height: 20px;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  color: #71717a;
  font-size: 11px;
}

.agent-label,
.version-state {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.version-state.warning {
  color: #b98538;
}

.version-state.success {
  color: #33a875;
}

.skill-card h2 {
  margin: 11px 0 6px;
  overflow: hidden;
  color: #18181b;
  font-size: 14px;
  font-weight: 650;
  line-height: 1.3;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.skill-card > p {
  display: -webkit-box;
  overflow: hidden;
  margin: 0;
  color: #62626b;
  font-size: 12px;
  line-height: 1.58;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.skill-tags,
.detail-tags {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 5px;
}

.skill-tags {
  margin-top: 11px;
}

.skill-tags span,
.detail-tags > span {
  display: inline-flex;
  min-height: 19px;
  align-items: center;
  gap: 4px;
  padding: 2px 7px;
  border-radius: 4px;
  background: #f4f4f5;
  color: #71717a;
  font-size: 9px;
}

.file-toggle {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 5px;
  margin-top: 14px;
  padding: 11px 0 0;
  border: 0;
  border-top: 1px solid #f0f0f2;
  background: transparent;
  color: #8a8a93;
  cursor: pointer;
  font-size: 11px;
  text-align: left;
}

.file-toggle:hover {
  color: #3f3f46;
}

.file-tree {
  margin: 7px -4px -3px;
}

.file-row {
  display: flex;
  width: 100%;
  height: 25px;
  align-items: center;
  gap: 5px;
  padding-right: 7px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: #73737c;
  cursor: pointer;
  font-size: 10px;
  text-align: left;
}

.file-row:hover,
.file-row.selected {
  background: #f1f1f3;
  color: #27272a;
}

.file-row > span:last-child {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-indent {
  display: inline-block;
  width: 11px;
}

.detail-panel {
  display: flex;
  flex-direction: column;
  background: #fff;
}

.detail-toolbar {
  display: flex;
  min-height: 44px;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 16px 0 18px;
  border-bottom: 1px solid #ececef;
}

.breadcrumb {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 3px;
  color: #71717a;
  font-size: 11px;
}

.breadcrumb > span,
.breadcrumb > strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.breadcrumb > strong {
  color: #3f3f46;
  font-weight: 600;
}

.crumb-separator {
  margin: 0 3px;
  color: #d4d4d8;
}

.detail-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 6px;
}

.toolbar-button {
  display: inline-flex;
  height: 30px;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: 1px solid #e4e4e7;
  border-radius: 6px;
  background: #fff;
  color: #52525b;
  cursor: pointer;
  font-size: 11px;
}

.toolbar-button:hover,
.toolbar-button.active {
  border-color: #d2d2d7;
  background: #f7f7f8;
  color: #18181b;
}

.toolbar-button:disabled {
  cursor: default;
  color: #b5b5bd;
}

.icon-only {
  width: 30px;
  padding: 0;
}

.app-picker {
  position: relative;
}

.app-select {
  min-width: 86px;
  padding: 0 8px;
}

.app-select span {
  max-width: 90px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-menu {
  position: absolute;
  z-index: 20;
  right: 0;
  top: calc(100% + 6px);
  width: 178px;
  padding: 5px;
  border: 1px solid #e1e1e5;
  border-radius: 8px;
  background: #fff;
  box-shadow: 0 10px 30px rgba(24, 24, 27, 0.13);
}

.app-menu button {
  display: flex;
  width: 100%;
  height: 31px;
  align-items: center;
  gap: 8px;
  padding: 0 9px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: #52525b;
  cursor: pointer;
  font-size: 11px;
  text-align: left;
}

.app-menu button:hover,
.app-menu button.selected {
  background: #f1f1f3;
  color: #18181b;
}

.action-strip {
  justify-content: space-between;
  border-bottom-color: #d8eee3;
  background: #f1faf5;
  color: #178453;
}

.action-strip.error {
  border-bottom-color: #f0d4d4;
  background: #fff5f5;
  color: #c24141;
}

.action-strip button {
  display: flex;
  align-items: center;
  border: 0;
  background: transparent;
  color: currentColor;
}

.detail-scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 0 24px 32px;
}

.detail-heading {
  padding: 17px 0 6px;
}

.detail-heading h1 {
  margin: 0 0 5px;
  color: #18181b;
  font-size: 25px;
  font-weight: 700;
  letter-spacing: -0.025em;
  line-height: 1.2;
}

.detail-tags > span {
  background: #f5f5f6;
  font-size: 10px;
}

.detail-tags .agent-chip {
  padding-left: 0;
  background: transparent;
  color: #52525b;
}

.warning-box {
  margin-bottom: 0;
  padding: 10px 12px;
  border: 1px solid #eedebc;
  border-radius: 6px;
  background: #fff8e8;
  color: #996b21;
  font-size: 11px;
}

.warning-box > div {
  display: flex;
  align-items: center;
  gap: 7px;
}

.warning-close {
  margin-left: auto;
  color: #bca77f;
}

.meta-bar {
  display: flex;
  align-items: center;
  gap: 13px;
  min-height: 44px;
  margin-bottom: 19px;
  border-bottom: 1px solid #eeeeef;
  color: #52525b;
  font-size: 10px;
}

.meta-bar span {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.meta-bar b {
  color: #a1a1aa;
  font-weight: 500;
}

.meta-bar i {
  width: 2px;
  height: 2px;
  border-radius: 50%;
  background: #c4c4ca;
}

.detail-content {
  max-width: 760px;
}

.content-loading {
  display: flex;
  min-height: 220px;
  align-items: center;
  justify-content: center;
  gap: 9px;
  color: #a1a1aa;
  font-size: 11px;
}

.source-view {
  min-height: 100%;
  margin: 0;
  padding: 16px;
  overflow: auto;
  border: 1px solid #e5e5e8;
  border-radius: 7px;
  background: #f8f8f9;
  color: #27272a;
  font-size: 12px;
  line-height: 1.65;
  white-space: pre-wrap;
}

.empty-state {
  display: flex;
  height: 100%;
  min-height: 240px;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 10px;
  padding: 28px;
  color: #c4c4ca;
  text-align: center;
}

.empty-state p {
  margin: 0;
  color: #8b8b94;
  font-size: 12px;
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
  color: #27272a;
  font-size: 13px;
  line-height: 1.65;
}

:deep(.markdown-body h1),
:deep(.markdown-body h2),
:deep(.markdown-body h3) {
  margin: 25px 0 9px;
  color: #18181b;
  font-weight: 650;
  letter-spacing: -0.012em;
  line-height: 1.3;
}

:deep(.markdown-body h1:first-child),
:deep(.markdown-body h2:first-child),
:deep(.markdown-body h3:first-child) {
  margin-top: 0;
}

:deep(.markdown-body h1) {
  font-size: 18px;
}

:deep(.markdown-body h2) {
  font-size: 15px;
}

:deep(.markdown-body h3) {
  font-size: 13px;
}

:deep(.markdown-body p) {
  margin: 7px 0;
}

:deep(.markdown-body ul),
:deep(.markdown-body ol) {
  margin: 8px 0;
  padding-left: 22px;
}

:deep(.markdown-body li) {
  margin: 5px 0;
  padding-left: 3px;
}

:deep(.markdown-body code) {
  padding: 2px 5px;
  border-radius: 4px;
  background: #f2f2f3;
  color: #3f3f46;
  font-size: 11px;
}

:deep(.markdown-body pre) {
  overflow: auto;
  margin: 16px 0;
  padding: 14px 15px;
  border: 1px solid #e5e5e8;
  border-radius: 7px;
  background: #f7f7f8;
  font-size: 11px;
  line-height: 1.6;
}

:deep(.markdown-body pre code) {
  padding: 0;
  background: transparent;
}

:deep(.markdown-body blockquote) {
  margin: 12px 0;
  padding-left: 13px;
  border-left: 2px solid #dedee2;
  color: #71717a;
}

</style>
