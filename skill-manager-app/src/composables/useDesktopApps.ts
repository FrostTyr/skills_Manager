import { computed, onMounted, ref } from 'vue'
import type { AppOption, Skill } from '@/types/skill'
import { getAvailableApps, openInApp, revealInFileManager } from '@/utils/tauri'

export function useDesktopApps(selectedSkill: () => Skill | null) {
  const availableApps = ref<AppOption[]>([])
  const selectedAppKey = ref('')
  const appMenuOpen = ref(false)
  const actionError = ref<string | null>(null)
  const actionNotice = ref<string | null>(null)

  const selectedApp = computed(
    () => availableApps.value.find((app) => app.key === selectedAppKey.value) ?? null,
  )
  const fileManagerApp = computed(
    () => availableApps.value.find((app) => app.kind === 'fileManager') ?? null,
  )

  onMounted(async () => {
    const apps = await getAvailableApps()
    availableApps.value = apps
    selectedAppKey.value =
      apps.find((app) => app.key === 'cursor')?.key ??
      apps.find((app) => app.kind !== 'fileManager')?.key ??
      apps[0]?.key ??
      ''
  })

  async function revealSelected() {
    const skill = selectedSkill()
    if (!skill) return
    await runAction(
      () => revealInFileManager(skill.path),
      `Opened in ${fileManagerApp.value?.label ?? 'file manager'}`,
    )
  }

  async function chooseAndOpenApp(app: AppOption) {
    selectedAppKey.value = app.key
    appMenuOpen.value = false
    const skill = selectedSkill()
    if (!skill) return
    await runAction(() => openInApp(skill.path, app.key), `Opened in ${app.label}`)
  }

  function clearActionMessage() {
    actionError.value = null
    actionNotice.value = null
  }

  async function runAction(fn: () => Promise<void>, success: string) {
    clearActionMessage()
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

  return {
    actionError,
    actionNotice,
    appMenuOpen,
    availableApps,
    chooseAndOpenApp,
    clearActionMessage,
    revealSelected,
    selectedApp,
    selectedAppKey,
  }
}
