<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import AppSidebar from '@/components/AppSidebar.vue'
import SkillDetailPanel from '@/components/SkillDetailPanel.vue'
import SkillListPanel from '@/components/SkillListPanel.vue'
import { useDesktopApps } from '@/composables/useDesktopApps'
import { useSkillFiles } from '@/composables/useSkillFiles'
import { useSkillStore } from '@/stores/skillStore'
import type { Skill, SkillFileEntry } from '@/types/skill'
import { renderMarkdown, renderSource } from '@/utils/markdown'

interface SidebarHandle {
  clearSearch: () => void
  focusSearch: () => void
}

const store = useSkillStore()
const sidebar = ref<SidebarHandle | null>(null)
const files = useSkillFiles()
const filteredSkills = computed(() => store.filteredSkills)
const selectedSkill = computed(() => store.selectedSkill)
const desktop = useDesktopApps(() => selectedSkill.value)

const selectedFileName = computed(
  () => files.selectedFile.value?.relativePath.split('/').pop() ?? '',
)
const isSkillIndexSelected = computed(
  () =>
    !files.selectedFile.value ||
    files.selectedFile.value.relativePath.toLowerCase() === 'skill.md',
)
const previewContent = computed(
  () => files.selectedFile.value?.content ?? selectedSkill.value?.body ?? '',
)
const renderedContent = computed(() => renderMarkdown(previewContent.value))
const highlightedContent = computed(() =>
  renderSource(previewContent.value, files.selectedFile.value?.language ?? 'markdown'),
)
const showSource = computed(
  () => store.detailViewMode === 'source' || !(files.selectedFile.value?.isMarkdown ?? true),
)
const issueSummary = computed(() => {
  if (store.errorMessage) return store.errorMessage
  if (store.issues.length === 0) return null
  return `${store.issues.length} scan issue${store.issues.length > 1 ? 's' : ''}`
})

async function selectSkill(skill: Skill) {
  store.selectedSkillId = skill.id
  files.resetSelection()
  await files.ensureFiles(skill)
}

async function selectFile(skill: Skill, file: SkillFileEntry) {
  if (store.selectedSkillId !== skill.id) store.selectedSkillId = skill.id
  await files.selectFile(skill, file)
}

function moveSelection(offset: number) {
  if (filteredSkills.value.length === 0) return
  const currentIndex = filteredSkills.value.findIndex(
    (skill) => skill.id === store.selectedSkillId,
  )
  const nextIndex =
    currentIndex === -1
      ? 0
      : Math.min(Math.max(currentIndex + offset, 0), filteredSkills.value.length - 1)
  const skill = filteredSkills.value[nextIndex]
  if (skill) void selectSkill(skill)
}

function onKeydown(event: KeyboardEvent) {
  if (event.metaKey && event.key.toLowerCase() === 'f') {
    event.preventDefault()
    sidebar.value?.focusSearch()
    return
  }
  if (event.metaKey && event.key.toLowerCase() === 'r') {
    event.preventDefault()
    void store.refresh()
    return
  }
  if (event.key === 'Escape') {
    desktop.appMenuOpen.value = false
    if (store.hasActiveFilter) {
      sidebar.value?.clearSearch()
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
    files.resetSelection()
    if (selectedSkill.value) await files.ensureFiles(selectedSkill.value)
  },
)

watch(files.errorMessage, (message) => {
  if (message) desktop.actionError.value = message
})

onMounted(async () => {
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('click', closeMenus)
  await store.refresh()
  if (selectedSkill.value) await files.ensureFiles(selectedSkill.value)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('click', closeMenus)
})

function closeMenus() {
  desktop.appMenuOpen.value = false
}
</script>

<template>
  <div class="app-shell">
    <AppSidebar ref="sidebar" :filtered-count="filteredSkills.length" />
    <SkillListPanel
      :collapsed-directories="files.collapsedDirectories.value"
      :directory-key="files.directoryKey"
      :expanded-skills="files.expandedSkills.value"
      :file-loading="files.fileLoading.value"
      :filtered-skills="filteredSkills"
      :issue-summary="issueSummary"
      :selected-file="files.selectedFile.value"
      :skill-file-count="files.skillFileCount"
      :visible-files="files.visibleFiles"
      @select-file="selectFile"
      @select-skill="selectSkill"
      @toggle-files="files.toggleSkillFiles"
    />
    <SkillDetailPanel
      :action-error="desktop.actionError.value"
      :action-notice="desktop.actionNotice.value"
      :app-menu-open="desktop.appMenuOpen.value"
      :available-apps="desktop.availableApps.value"
      :highlighted-content="highlightedContent"
      :is-skill-index-selected="isSkillIndexSelected"
      :rendered-content="renderedContent"
      :selected-app="desktop.selectedApp.value"
      :selected-app-key="desktop.selectedAppKey.value"
      :selected-file="files.selectedFile.value"
      :selected-file-loading="files.selectedFileLoading.value"
      :selected-file-name="selectedFileName"
      :selected-skill="selectedSkill"
      :show-source="showSource"
      @choose-app="desktop.chooseAndOpenApp"
      @clear-message="desktop.clearActionMessage"
      @reveal="desktop.revealSelected"
      @toggle-app-menu="desktop.appMenuOpen.value = !desktop.appMenuOpen.value"
    />
  </div>
</template>
