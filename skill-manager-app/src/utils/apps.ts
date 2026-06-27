import type { AppOption } from '@/types/skill'

const APP_ICONS: Record<string, string> = {
  cursor: '/apps/cursor.ico',
  vscode: '/apps/vscode.ico',
  trae: '/apps/trae.png',
  sublime: '/apps/sublime.ico',
  notepadpp: '/apps/notepadpp.ico',
  'windows-terminal': '/apps/windows-terminal.ico',
  powershell: '/apps/powershell.png',
  warp: '/apps/warp.png',
  ghostty: '/apps/ghostty.ico',
  terminal: '/apps/terminal.svg',
  'file-manager': '/apps/file-explorer.png',
}

const KIND_ICONS: Record<AppOption['kind'], string> = {
  editor: '/apps/editor.svg',
  terminal: '/apps/terminal.svg',
  fileManager: '/apps/file-explorer.png',
}

export function appIcon(app: AppOption | null): string {
  if (!app) return KIND_ICONS.editor
  return APP_ICONS[app.key] ?? KIND_ICONS[app.kind]
}