import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { ScanResult } from '@/types/skill'
import { scanSkills } from '@/utils/tauri'
import { useSkillStore } from './skillStore'

vi.mock('@/utils/tauri', () => ({
  scanSkills: vi.fn(),
}))

const scanSkillsMock = vi.mocked(scanSkills)

describe('skill store agent visibility', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    scanSkillsMock.mockReset()
  })

  it('returns only installed agents for the sidebar', () => {
    const store = useSkillStore()
    store.agents = [
      { key: 'codex', label: 'Codex', path: '~/.codex/skills', exists: true, skillCount: 1 },
      {
        key: 'openclaw',
        label: 'OpenClaw',
        path: '~/.openclaw/skills',
        exists: false,
        skillCount: 0,
      },
    ]

    expect(store.visibleAgents.map((agent) => agent.key)).toEqual(['codex'])
  })

  it('removes hidden agents from the active filter after refresh', async () => {
    const store = useSkillStore()
    store.selectedAgents = ['openclaw', 'codex']
    scanSkillsMock.mockResolvedValue(scanResult())

    await store.refresh()

    expect(store.selectedAgents).toEqual(['codex'])
  })
})

function scanResult(): ScanResult {
  return {
    skills: [],
    agents: [
      { key: 'codex', label: 'Codex', path: '~/.codex/skills', exists: true, skillCount: 0 },
      {
        key: 'openclaw',
        label: 'OpenClaw',
        path: '~/.openclaw/skills',
        exists: false,
        skillCount: 0,
      },
    ],
    issues: [],
    durationMs: 8,
  }
}
