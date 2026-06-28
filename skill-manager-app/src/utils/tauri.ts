import { invoke } from '@tauri-apps/api/core'
import type {
  AppOption,
  ScanResult,
  SkillFileContent,
  SkillFileEntry,
} from '@/types/skill'

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

const inTauri = () => Boolean(window.__TAURI_INTERNALS__)

const mockSkills: ScanResult = {
  skills: [
    {
      id: 'openclaw:1password',
      name: '1password',
      path: '/mock/1password',
      realPath: '/mock/1password',
      isSymlink: false,
      isBrokenLink: false,
      description: 'Set up and use 1Password CLI for sign-in, desktop integration, and secret retrieval.',
      version: '1.0',
      category: 'other',
      customTags: [],
      sourceAgents: ['openclaw'],
      sourceAgentLabels: ['OpenClaw'],
      body: `# Overview

Set up and use 1Password CLI for sign-in, desktop integration, and secret retrieval. Follow the official CLI get-started steps. Don't guess install commands.

## References

- \`references/get-started.md\` - install + integration
- \`references/cli-examples.md\` - real \`op\` examples

## Workflow

1. Check OS + shell
2. Verify CLI present: \`op --version\`
3. Confirm desktop app integration is enabled
4. Run \`op signin\` inside a dedicated tmux session
5. Verify: \`op whoami\`

\`\`\`sh
SOCKET_DIR="\${OPENCLAW_TMUX_SOCKET_DIR:-\${HOME}/.openclaw/tmux}"
mkdir -p "$SOCKET_DIR"
\`\`\``,
      warnings: [],
    },
    {
      id: 'claude:daily-log',
      name: '每日工作日志',
      path: '/mock/daily-log',
      realPath: '/mock/daily-log',
      isSymlink: false,
      isBrokenLink: false,
      description: 'Helps record work logs, track project progress, and generate daily and weekly summaries.',
      version: 'Unknown',
      category: 'claude',
      customTags: [],
      sourceAgents: ['claude'],
      sourceAgentLabels: ['Claude CLI'],
      body: '# Daily work log\n\nRecord progress, risks, and next steps.',
      warnings: [],
    },
    {
      id: 'claude:env',
      name: '配置主要环境管理',
      path: '/mock/env',
      realPath: '/mock/env',
      isSymlink: false,
      isBrokenLink: false,
      description: 'Manages development environment settings, including variables, dependency versions, and toolchains.',
      version: '1.2',
      category: 'system',
      customTags: ['other'],
      sourceAgents: ['claude'],
      sourceAgentLabels: ['Claude CLI'],
      body: '# Environment management\n\nManage local development environment settings.',
      warnings: [],
    },
    {
      id: 'hermes:airtable',
      name: 'airtable',
      path: '/mock/airtable',
      realPath: '/mock/airtable',
      isSymlink: false,
      isBrokenLink: false,
      description: 'Airtable MCP: Records, filters, queries, views, and base management via API.',
      version: '2.0',
      category: 'other',
      customTags: [],
      sourceAgents: ['hermes'],
      sourceAgentLabels: ['Hermes'],
      body: '# Airtable\n\nManage records, views, filters, and schemas.',
      warnings: [],
    },
  ],
  agents: [
    { key: 'hermes', label: 'Hermes', path: '~/.hermes/skills', exists: true, skillCount: 157 },
    { key: 'codex', label: 'Codex', path: '~/.codex/skills', exists: true, skillCount: 5 },
    { key: 'claude', label: 'Claude CLI', path: '~/.claude/skills', exists: true, skillCount: 26 },
    {
      key: 'openclaw',
      label: 'OpenClaw',
      path: '~/.openclaw/skills',
      exists: true,
      skillCount: 93,
    },
  ],
  issues: [],
  durationMs: 38,
}

const mockFiles: Record<string, SkillFileEntry[]> = {
  '/mock/1password': [
    { relativePath: 'references', name: 'references', isDirectory: true, depth: 0 },
    {
      relativePath: 'references/cli-examples.md',
      name: 'cli-examples.md',
      isDirectory: false,
      depth: 1,
    },
    {
      relativePath: 'references/get-started.md',
      name: 'get-started.md',
      isDirectory: false,
      depth: 1,
    },
    { relativePath: 'SKILL.md', name: 'SKILL.md', isDirectory: false, depth: 0 },
    { relativePath: 'README.md', name: 'README.md', isDirectory: false, depth: 0 },
  ],
  '/mock/daily-log': [
    { relativePath: 'templates', name: 'templates', isDirectory: true, depth: 0 },
    { relativePath: 'templates/daily.md', name: 'daily.md', isDirectory: false, depth: 1 },
    { relativePath: 'templates/weekly.md', name: 'weekly.md', isDirectory: false, depth: 1 },
    { relativePath: 'SKILL.md', name: 'SKILL.md', isDirectory: false, depth: 0 },
  ],
  '/mock/env': [
    { relativePath: 'SKILL.md', name: 'SKILL.md', isDirectory: false, depth: 0 },
    { relativePath: 'config.json', name: 'config.json', isDirectory: false, depth: 0 },
  ],
  '/mock/airtable': [
    { relativePath: 'SKILL.md', name: 'SKILL.md', isDirectory: false, depth: 0 },
    { relativePath: 'README.md', name: 'README.md', isDirectory: false, depth: 0 },
    { relativePath: 'examples.md', name: 'examples.md', isDirectory: false, depth: 0 },
    { relativePath: 'schema.json', name: 'schema.json', isDirectory: false, depth: 0 },
  ],
}

export async function scanSkills(): Promise<ScanResult> {
  return inTauri() ? invoke<ScanResult>('scan_skills') : mockSkills
}

export async function listSkillFiles(path: string): Promise<SkillFileEntry[]> {
  return inTauri() ? invoke<SkillFileEntry[]>('list_skill_files', { path }) : (mockFiles[path] ?? [])
}

export async function readSkillFile(
  path: string,
  relativePath: string,
): Promise<SkillFileContent> {
  if (inTauri()) {
    return invoke<SkillFileContent>('read_skill_file', { path, relativePath })
  }

  const skill = mockSkills.skills.find((item) => item.path === path)
  const content =
    relativePath === 'SKILL.md'
      ? skill?.body ?? ''
      : relativePath.endsWith('.json')
        ? '{\n  "enabled": true,\n  "source": "skill-manager"\n}'
        : `# ${relativePath.split('/').pop()}\n\nPreview content for ${relativePath}.`

  return {
    relativePath,
    content,
    language: relativePath.endsWith('.json') ? 'json' : 'markdown',
    isMarkdown: !relativePath.endsWith('.json'),
    size: content.length,
  }
}

export async function getAvailableApps(): Promise<AppOption[]> {
  if (inTauri()) return invoke<AppOption[]>('available_apps')
  return [
    { key: 'cursor', label: 'Cursor', kind: 'editor' },
    { key: 'vscode', label: 'VS Code', kind: 'editor' },
    { key: 'sublime', label: 'Sublime Text', kind: 'editor' },
    { key: 'terminal', label: 'Terminal', kind: 'terminal' },
    { key: 'file-manager', label: 'File Manager', kind: 'fileManager' },
  ]
}

export async function revealInFileManager(path: string): Promise<void> {
  if (inTauri()) await invoke('reveal_in_file_manager', { path })
}

export async function openInApp(path: string, app: string): Promise<void> {
  if (inTauri()) await invoke('open_in_app', { path, app })
}
