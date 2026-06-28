import { onBeforeUnmount, onMounted } from 'vue'

interface KeyboardShortcutsOptions {
  clearFilters: () => void
  clearSearch: () => void
  closeMenus: () => void
  focusSearch: () => void
  hasActiveFilter: () => boolean
  moveSelection: (offset: number) => void
  refresh: () => void | Promise<void>
}

export function useKeyboardShortcuts(options: KeyboardShortcutsOptions) {
  function onKeydown(event: KeyboardEvent) {
    if (event.metaKey && event.key.toLowerCase() === 'f') {
      event.preventDefault()
      options.focusSearch()
      return
    }

    if (event.metaKey && event.key.toLowerCase() === 'r') {
      event.preventDefault()
      void options.refresh()
      return
    }

    if (event.key === 'Escape') {
      options.closeMenus()
      if (options.hasActiveFilter()) {
        options.clearSearch()
        options.clearFilters()
      }
      return
    }

    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) return

    if (event.key === 'ArrowDown') {
      event.preventDefault()
      options.moveSelection(1)
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      options.moveSelection(-1)
    }
  }

  onMounted(() => window.addEventListener('keydown', onKeydown))
  onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
}
