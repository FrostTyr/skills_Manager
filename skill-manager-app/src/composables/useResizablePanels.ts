import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

type PanelKey = 'sidebar' | 'list'

interface PanelSizeConfig {
  default: number
  min: number
  max: number
}

interface StoredPanelWidths {
  sidebar?: number
  list?: number
}

const STORAGE_KEY = 'skill-manager.panel-widths.v1'
const DETAIL_MIN_WIDTH = 420
const RESIZER_WIDTH = 8

const PANEL_SIZES: Record<PanelKey, PanelSizeConfig> = {
  sidebar: { default: 300, min: 240, max: 420 },
  list: { default: 520, min: 360, max: 720 },
}

export function useResizablePanels() {
  const sidebarWidth = ref(PANEL_SIZES.sidebar.default)
  const listWidth = ref(PANEL_SIZES.list.default)
  const activePanel = ref<PanelKey | null>(null)
  let dragStartX = 0
  let dragStartWidth = 0
  let previousCursor = ''
  let previousUserSelect = ''

  const shellStyle = computed(() => ({
    '--sidebar-width': `${sidebarWidth.value}px`,
    '--list-width': `${listWidth.value}px`,
  }))

  function startResize(panel: PanelKey, event: PointerEvent) {
    activePanel.value = panel
    dragStartX = event.clientX
    dragStartWidth = panel === 'sidebar' ? sidebarWidth.value : listWidth.value
    previousCursor = document.body.style.cursor
    previousUserSelect = document.body.style.userSelect
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
    window.addEventListener('pointermove', onPointerMove)
    window.addEventListener('pointerup', stopResize)
    window.addEventListener('pointercancel', stopResize)
  }

  function onPointerMove(event: PointerEvent) {
    const panel = activePanel.value
    if (!panel) return

    const nextWidth = dragStartWidth + event.clientX - dragStartX
    setPanelWidth(panel, nextWidth)
  }

  function stopResize() {
    if (!activePanel.value) return

    activePanel.value = null
    document.body.style.cursor = previousCursor
    document.body.style.userSelect = previousUserSelect
    window.removeEventListener('pointermove', onPointerMove)
    window.removeEventListener('pointerup', stopResize)
    window.removeEventListener('pointercancel', stopResize)
    saveWidths()
  }

  function setPanelWidth(panel: PanelKey, width: number) {
    const clamped = clampPanelWidth(panel, width)
    if (panel === 'sidebar') {
      sidebarWidth.value = clamped
    } else {
      listWidth.value = clamped
    }
    saveWidths()
  }

  function clampPanelWidth(panel: PanelKey, width: number) {
    const config = PANEL_SIZES[panel]
    const otherWidth = panel === 'sidebar' ? listWidth.value : sidebarWidth.value
    const viewportMax = window.innerWidth - otherWidth - DETAIL_MIN_WIDTH - RESIZER_WIDTH * 2
    const max = Math.max(config.min, Math.min(config.max, viewportMax))
    return Math.round(Math.min(Math.max(width, config.min), max))
  }

  function loadWidths() {
    const stored = readStoredWidths()
    sidebarWidth.value = clampPanelWidth('sidebar', stored.sidebar ?? PANEL_SIZES.sidebar.default)
    listWidth.value = clampPanelWidth('list', stored.list ?? PANEL_SIZES.list.default)
  }

  function readStoredWidths(): StoredPanelWidths {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY)
      return raw ? JSON.parse(raw) : {}
    } catch {
      return {}
    }
  }

  function saveWidths() {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        sidebar: sidebarWidth.value,
        list: listWidth.value,
      }),
    )
  }

  function clampToViewport() {
    sidebarWidth.value = clampPanelWidth('sidebar', sidebarWidth.value)
    listWidth.value = clampPanelWidth('list', listWidth.value)
    saveWidths()
  }

  onMounted(() => {
    loadWidths()
    window.addEventListener('resize', clampToViewport)
  })

  onBeforeUnmount(() => {
    stopResize()
    window.removeEventListener('resize', clampToViewport)
  })

  return {
    activePanel,
    listWidth,
    shellStyle,
    sidebarWidth,
    startResize,
  }
}
