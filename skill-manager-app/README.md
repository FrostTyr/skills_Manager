# Skill Manager

基于 Tauri 2、Rust、Vue 3 和 TypeScript 的只读 AI Agent Skill 管理器。

## 工程结构

```text
src/
├── components/       页面级展示组件
├── composables/      文件浏览、桌面应用等交互逻辑
├── stores/           Pinia 领域状态与筛选规则
├── types/            前后端 IPC 数据契约
├── utils/            Tauri 客户端、Markdown 和纯函数
└── styles/           设计令牌与全局样式

src-tauri/src/
├── commands.rs       仅负责 Tauri IPC 参数编排
├── services/         文件读取等应用服务
├── state.rs          扫描结果路径白名单状态
├── scanner/          Skill 发现、解析和去重
├── platform/         macOS、Linux、Windows 平台适配
└── models/           IPC 输出模型
```

依赖方向保持为：组件 → composable/store → IPC client；Tauri command → service/state → scanner/platform。

## 开发

```bash
npm install
npm run dev
npm run tauri:dev
```

## 质量检查

```bash
npm run check
cd src-tauri
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 设计约束

- 产品保持只读，不执行 Skill 中的脚本。
- 文件预览只能访问最近一次扫描产生的路径白名单。
- Markdown 禁止原始 HTML，并在渲染后再次消毒。
- 新交互逻辑优先进入 composable；组件保持展示和事件编排职责。
- 新文件系统能力优先进入 `services/`；`commands.rs` 不承载遍历或解析逻辑。

## Codex 扫描来源

macOS 与 Windows 均以当前用户主目录为基准，Codex Skills 包含：

- `~/.codex/skills`，包括 `.system` 内置 Skills。
- `~/.agents/skills` 共享 Skills。
- `codex plugin list --json` 返回的已安装、已启用插件中的 Skills。
- Codex CLI 不可用时，回退到 `~/.codex/plugins/cache` 中每个插件的最新缓存版本。

扫描结果最终按真实文件路径去重，避免软链接或多来源造成重复展示。
