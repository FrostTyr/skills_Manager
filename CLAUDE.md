# AI Agent Skill Manager

**Version**: v0.3.1  
**Tech Stack**: Tauri v2 + Rust + Vue 3 + TypeScript + Vite  
**Platform**: macOS + Windows 10/11 x64

## Project Overview

A desktop tool for managing Skills across multiple AI Agents (Hermes, Codex, Claude, OpenClaw). Provides unified scanning, viewing, and management of Skills scattered across different agent directories.

## Current Status

🚧 **v0.3.1**
- Cross-platform Skills scanning and path validation
- Platform-specific file manager, editor, and terminal integration
- Windows x64 NSIS packaging and CI
- Product remains read-only; install/uninstall/sync are not part of v0.3.1

## Architecture

```
Tauri App
├── Rust Backend (src-tauri/)
│   ├── Scanner: File system traversal, symlink resolution
│   ├── Parser: YAML frontmatter + Markdown body
│   ├── Commands: Tauri IPC handlers
│   └── OpenClaw Loader: CLI integration for actual skill list
└── Vue 3 Frontend (src/)
    ├── Stores (Pinia): skillStore, filterStore
    ├── Components: Sidebar, SkillList, DetailPanel
    └── Utils: Markdown rendering, sanitization
```

## Key Directories

- `src-tauri/src/` - Rust backend
  - `scanner/mod.rs` - Core scanning engine
  - `commands.rs` - Tauri command handlers
  - `models/skill.rs` - Skill data structure
- `src/` - Vue 3 frontend
  - `stores/` - Pinia state management
  - `types/` - TypeScript definitions
- `skill-manager-app/` - Main application directory

## Development Commands

```bash
# Frontend dev server
npm run dev

# Tauri desktop app (hot reload)
npm run tauri:dev

# Build production
npm run tauri:build

# Run tests
cargo test          # Rust tests (7 tests)
npm run test        # Frontend tests (2 tests)
```

## Build Output

Production build generates:
- **App Bundle**: `src-tauri/target/release/bundle/macos/Skill Manager.app` (8.7MB)
- **DMG Installer**: `src-tauri/target/release/bundle/dmg/Skill Manager_0.1.0_aarch64.dmg` (3.3MB)

## Agent Scanning Rules

**Default Paths**:
- Hermes: `~/.hermes/skills/`
- Codex: `~/.codex/skills/`
- Claude: `~/.claude/skills/`
- OpenClaw: Uses `openclaw skills list --eligible --json` for actual loaded skills

**Deduplication**: Skills with same `real_path` are merged, `source_agents` aggregates all sources.

**OpenClaw Special**: Matches exact `source` from CLI output to show only the version OpenClaw actually loads.

## Code Style

**Rust**:
- Use `thiserror` for error types
- `serde_yml` for YAML parsing
- Avoid unwrap(), use `?` operator
- Unit tests in same file with `#[cfg(test)]`

**Vue/TypeScript**:
- Composition API with `<script setup lang="ts">`
- Pinia for state management
- CSS variables in `tokens.css` for theming
- No heavy UI libraries (Element Plus, Ant Design)

**Security**:
- Markdown: `html: false` + DOMPurify sanitization
- Path operations: Whitelist validation via `ScannedPaths`
- No script execution from Skills

## Current Capabilities

✅ **Implemented**:
- Rust scanning engine with symlink resolution
- Cross-agent deduplication
- OpenClaw loader integration
- Vue 3 UI with search, filtering, detail panel
- Markdown rendering with syntax highlighting
- Desktop integration (Finder/Explorer and supported editors/terminals)
- Security hardening (CSP, path whitelist)
- macOS packaging (.dmg + .app)

📋 **Known Limitations**:
- No E2E tests (Playwright)
- No Markdown render caching
- No virtual scrolling for large lists
- App.vue needs component splitting (1039 lines)

## Future Roadmap

- Install/uninstall Skills (symlink management)
- Version detection and update prompts
- File watching with auto-refresh
- Custom agent directory configuration
- Component splitting (Sidebar, SkillList, SkillDetail)
- E2E tests (Playwright)
- Markdown render caching
- Virtual scrolling for large lists

## Documentation

- `skills-manager-prd.md` - Product requirements
- `skills-manager-design-doc.md` - UI/UX design
- `skills-manager-technical-architecture.md` - Technical details

## Working with This Project

**Before making changes**:
1. Read relevant docs above
2. Run `cargo test` and `npm run test`

**When adding features**:
- Update PRD if changing requirements
- Add tests for new Rust functions

**When fixing bugs**:
- Check if issue is documented in code review reports
- Add regression test if applicable
- Verify fix doesn't break existing tests

## ⚠️ Git Workflow Rules

**IMPORTANT - DO NOT AUTO-COMMIT OR AUTO-PUSH**:
1. **Never automatically commit** changes to git
2. **Never automatically push** to GitHub
3. All changes require user review and manual approval
4. User will manually run `git add`, `git commit`, and `git push`

**What to push**:
- ✅ Source code in `skill-manager-app/` directory
- ❌ Documentation files (*.md) - keep local only

**Workflow**:
1. Make code changes
2. Run tests to verify
3. Report changes to user
4. Wait for user to manually commit and push
