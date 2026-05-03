import { invoke } from '@tauri-apps/api/core'
import type { ScanResult } from '@/types/skill'

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

const inTauri = () => Boolean(window.__TAURI_INTERNALS__)

export async function scanSkills(): Promise<ScanResult> {
  if (inTauri()) {
    return invoke<ScanResult>('scan_skills')
  }

  return {
    skills: [],
    agents: [
      {
        key: 'hermes',
        label: 'Hermes',
        path: '~/.hermes/skills',
        exists: false,
        skillCount: 0,
      },
      {
        key: 'codex',
        label: 'Codex',
        path: '~/.codex/skills',
        exists: false,
        skillCount: 0,
      },
      {
        key: 'claude',
        label: 'Claude',
        path: '~/.claude/skills',
        exists: false,
        skillCount: 0,
      },
      {
        key: 'openclaw',
        label: 'OpenClaw',
        path: '~/.openclaw/skills',
        exists: false,
        skillCount: 0,
      },
    ],
    issues: [
      {
        path: '',
        level: 'warning',
        message: 'Frontend preview is running outside Tauri; real file scanning is unavailable.',
      },
    ],
    durationMs: 0,
  }
}

export async function revealInFinder(path: string): Promise<void> {
  await invoke('reveal_in_finder', { path })
}

export async function openInEditor(path: string, editor: 'cursor' | 'vscode'): Promise<void> {
  await invoke('open_in_editor', { path, editor })
}
