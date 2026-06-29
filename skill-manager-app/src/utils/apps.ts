import type { AppOption } from '@/types/skill'

const APP_ICONS: Record<string, string> = {
  cursor: '/apps/cursor.png',
  vscode: '/apps/vscode.png',
  trae: '/apps/trae.png',
  sublime: '/apps/sublime.png',
  notepadpp: '/apps/notepadpp.ico',
  'windows-terminal': '/apps/windows-terminal.ico',
  powershell: '/apps/powershell.png',
  warp: '/apps/warp.png',
  ghostty: '/apps/ghostty.png',
  terminal: '/apps/terminal.png',
  finder: '/apps/finder.png',
  'file-manager': '/apps/file-explorer.png',
}

const KIND_ICONS: Record<AppOption['kind'], string> = {
  editor: '/apps/editor.svg',
  terminal: '/apps/terminal.png',
  fileManager: '/apps/file-explorer.png',
}

export function appIcon(app: AppOption | null): string {
  if (!app) return KIND_ICONS.editor
  return APP_ICONS[app.key] ?? KIND_ICONS[app.kind]
}
