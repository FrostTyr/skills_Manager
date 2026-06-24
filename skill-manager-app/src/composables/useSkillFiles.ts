import { ref } from 'vue'
import type { Skill, SkillFileContent, SkillFileEntry } from '@/types/skill'
import { listSkillFiles, readSkillFile } from '@/utils/tauri'

export function useSkillFiles() {
  const fileLists = ref<Record<string, SkillFileEntry[]>>({})
  const fileLoading = ref(new Set<string>())
  const expandedSkills = ref(new Set<string>())
  const collapsedDirectories = ref(new Set<string>())
  const selectedFile = ref<SkillFileContent | null>(null)
  const selectedFileLoading = ref(false)
  const errorMessage = ref<string | null>(null)

  async function ensureFiles(skill: Skill) {
    if (fileLists.value[skill.id] || fileLoading.value.has(skill.id)) return

    fileLoading.value = withSetItem(fileLoading.value, skill.id, true)
    try {
      const files = await listSkillFiles(skill.path)
      fileLists.value = { ...fileLists.value, [skill.id]: files }
    } catch (error) {
      errorMessage.value = toErrorMessage(error)
    } finally {
      fileLoading.value = withSetItem(fileLoading.value, skill.id, false)
    }
  }

  async function toggleSkillFiles(skill: Skill) {
    const shouldExpand = !expandedSkills.value.has(skill.id)
    expandedSkills.value = withSetItem(expandedSkills.value, skill.id, shouldExpand)
    if (shouldExpand) await ensureFiles(skill)
  }

  function toggleDirectory(skillId: string, path: string) {
    const key = directoryKey(skillId, path)
    collapsedDirectories.value = withSetItem(
      collapsedDirectories.value,
      key,
      !collapsedDirectories.value.has(key),
    )
  }

  function visibleFiles(skill: Skill) {
    return (fileLists.value[skill.id] ?? []).filter((file) => {
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

  function skillFileCount(skill: Skill) {
    return (fileLists.value[skill.id] ?? []).filter((file) => !file.isDirectory).length
  }

  async function selectFile(skill: Skill, file: SkillFileEntry) {
    if (file.isDirectory) {
      toggleDirectory(skill.id, file.relativePath)
      return
    }

    selectedFileLoading.value = true
    errorMessage.value = null
    try {
      selectedFile.value = await readSkillFile(skill.path, file.relativePath)
    } catch (error) {
      selectedFile.value = {
        relativePath: file.relativePath,
        content: toErrorMessage(error),
        language: 'plaintext',
        isMarkdown: false,
        size: 0,
      }
    } finally {
      selectedFileLoading.value = false
    }
  }

  function resetSelection() {
    selectedFile.value = null
  }

  return {
    collapsedDirectories,
    directoryKey,
    ensureFiles,
    errorMessage,
    expandedSkills,
    fileLoading,
    resetSelection,
    selectFile,
    selectedFile,
    selectedFileLoading,
    skillFileCount,
    toggleDirectory,
    toggleSkillFiles,
    visibleFiles,
  }
}

function directoryKey(skillId: string, path: string) {
  return `${skillId}:${path}`
}

function withSetItem<T>(source: Set<T>, item: T, included: boolean): Set<T> {
  const next = new Set(source)
  if (included) next.add(item)
  else next.delete(item)
  return next
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}
