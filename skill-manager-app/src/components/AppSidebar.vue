<script setup lang="ts">
import { Hash, RefreshCw, Search, X } from 'lucide-vue-next'
import { onBeforeUnmount, ref } from 'vue'
import { useSkillStore } from '@/stores/skillStore'
import { agentIcon } from '@/utils/agents'

defineProps<{ filteredCount: number }>()

const store = useSkillStore()
const searchInput = ref<HTMLInputElement | null>(null)
const searchDebounceTimer = ref<ReturnType<typeof setTimeout> | null>(null)

function onSearchInput(event: Event) {
  const value = (event.target as HTMLInputElement).value
  if (searchDebounceTimer.value !== null) clearTimeout(searchDebounceTimer.value)
  searchDebounceTimer.value = setTimeout(() => {
    store.searchQuery = value
  }, 180)
}

function clearSearch() {
  if (searchDebounceTimer.value !== null) clearTimeout(searchDebounceTimer.value)
  store.searchQuery = ''
  if (searchInput.value) searchInput.value.value = ''
  store.ensureSelection()
}

function focusSearch() {
  searchInput.value?.focus()
}

defineExpose({ clearSearch, focusSearch })

onBeforeUnmount(() => {
  if (searchDebounceTimer.value !== null) clearTimeout(searchDebounceTimer.value)
})
</script>

<template>
  <aside class="sidebar">
    <header class="brand">
      <img class="app-logo" src="/icon.svg" alt="" aria-hidden="true" />
      <div class="brand-copy">
        <h1>Skill Manager</h1>
        <span>v0.2.1</span>
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
        <strong>{{ filteredCount }}</strong>
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
</template>
