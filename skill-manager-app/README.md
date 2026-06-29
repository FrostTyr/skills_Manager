# Skill Manager

Skill Manager is a read-only desktop manager for AI Agent skills, built with Tauri 2, Rust, Vue 3, and TypeScript. It scans local skill directories, displays metadata, previews safe text files, and opens skill folders in supported editors or file managers.

## Stack

- Desktop shell: Tauri 2
- Backend: Rust 2021
- Frontend: Vue 3, Pinia, TypeScript, Vite
- Content rendering: markdown-it, highlight.js, DOMPurify
- Tests: Vitest, Cargo test

## Project Structure

```text
src/
  components/       Page-level presentation components
  composables/      File browsing and desktop-app interaction logic
  stores/           Pinia domain state and filtering rules
  types/            Frontend/backend IPC data contracts
  utils/            Tauri client helpers, Markdown rendering, pure utilities
  styles/           Design tokens and global styles

src-tauri/src/
  commands.rs       Tauri IPC argument orchestration
  services/         Application services such as file reads
  state.rs          Whitelist state for paths from the latest scan
  scanner/          Skill discovery, parsing, and deduplication
  platform/         macOS, Linux, and Windows platform adapters
  models/           IPC output models
```

Dependency direction stays one-way: components -> composables/stores -> IPC client; Tauri commands -> services/state -> scanner/platform.

## Local Development

```bash
npm install
npm run dev
npm run tauri:dev
```

Common commands:

```bash
npm run test
npm run build
npm run check:frontend
npm run check:rust
npm run check
npm run tauri:build
```

## Downloads

Versioned desktop installers are published from git tags through GitHub Releases:

- Releases page: <https://github.com/FrostTyr/skills_Manager/releases>
- Windows: `Skill-Manager_<version>_windows_x64-setup.exe`
- macOS: `Skill-Manager_<version>_macos_<arch>.dmg`
- Release metadata: each release includes `release-manifest.json` with the release date, assets, download URLs, file sizes, and SHA256 hashes.

## Quality Gates

Run this before submitting changes:

```bash
npm run check
```

The gate runs:

- Version consistency checks across `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`
- Frontend unit tests and production build
- Rust formatting check
- Rust Clippy with warnings denied
- Rust unit tests

The code quality workflow lives at `.github/workflows/ci.yml` and runs equivalent checks on pushes to `main` and pull requests. The desktop workflow lives at `.github/workflows/desktop-ci.yml`; it runs platform regression checks, builds Windows and macOS installers, uploads CI artifacts, and publishes GitHub Releases for `v*` tags.

## Engineering Constraints

- The product is read-only and must not execute scripts from skills.
- File preview may only access the path whitelist produced by the latest scan.
- File reads must validate relative paths, root boundaries, size limits, binary content, and UTF-8 content.
- Markdown disables raw HTML and is sanitized with DOMPurify after rendering.
- New interaction logic should live in composables; components should focus on presentation and event orchestration.
- New filesystem capabilities should live in `services/`; `commands.rs` should not contain traversal or parsing logic.
- Tauri capability files live in `src-tauri/capabilities/`; permission changes should be reviewed with the code that needs them.

## Codex Scan Sources

On macOS and Windows, Codex skill scanning is based on the current user's home directory and includes:

- `~/.codex/skills`, including built-in `.system` skills.
- `~/.agents/skills` shared skills.
- Skills from installed and enabled plugins returned by `codex plugin list --json`.
- When the Codex CLI is unavailable, the latest cached plugin versions under `~/.codex/plugins/cache`.

Scan results are deduplicated by real filesystem path to avoid duplicate display from symlinks or multiple sources.

## Release Checks

Before releasing, make sure the version is identical in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`, then run:

```bash
npm run check
npm run tauri:build -- --bundles dmg
```

Windows-specific installer configuration lives in `src-tauri/tauri.windows.conf.json`. Windows installers are built on the `windows-latest` GitHub Actions runner with the `x86_64-pc-windows-msvc` target.

To publish a version, create and push a matching tag from the release commit:

```bash
git tag v0.3.1
git push origin v0.3.1
```

The tag version must match `package.json`; the release job will stop if the tag and package version differ. GitHub Releases is the source of truth for installer update dates. The generated `release-manifest.json` mirrors that date in its `releaseDate` field and lists every uploaded installer with its SHA256 hash.

If installer behavior, artifact names, or supported platforms change, update this README and `.github/workflows/desktop-ci.yml` in the same change.
