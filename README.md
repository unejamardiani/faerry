# Faerry

Faerry is a desktop app for carrying a portable `.agents` setup across agent
tools. It keeps one repository as the source of truth for global instructions,
skills, MCP servers, and optional source repositories, then links or syncs that
setup into tools such as Claude Code, Codex, OpenCode, GitHub Copilot CLI, and
Claude Desktop skill packages.

Faerry combines the practical idea of a ferry with a small hint of faery-like
magic. The "ae" spelling is a personal signature, while the product remains a
general-purpose portable agents manager.

## Status

Faerry is early software. The app currently focuses on local desktop workflows
for a portable `.agents` repository:

- inspect repo health, skills, designs, MCP registries, and source repos
- preview link and MCP changes before applying them
- link `~/.agents` into supported CLI agent tools
- aggregate skills from local or Git sources through `sources.json`
- package Claude Desktop skill ZIPs
- import portable agents repos from Git URLs, ZIP URLs, or local ZIP files

## Portable Agents Repo Contract

Faerry manages a repository with this shape:

```text
.
├── AGENTS.md
├── skills/
├── mcp/
│   └── servers.json
└── sources.json       # optional
```

`AGENTS.md` and `skills/` are required. `mcp/servers.json` is required for MCP
sync. `sources.json` is optional and lets a home repo load skills, commands, and
`DESIGN.md` files from other local or Git repositories.

Generated source caches and aggregate skills live under `.agents-manager/` in
the managed repo. That folder is runtime state and should not be committed.

## Development

Requirements:

- Node.js 20 or newer
- Rust toolchain
- platform prerequisites for Tauri 2

Install dependencies:

```bash
npm install
```

Run the desktop app:

```bash
npm run dev
```

Check TypeScript and Rust:

```bash
npm run check
```

Build:

```bash
npm run build
```

Create a portable package for the current platform:

```bash
npm run package:portable
```

## Bundled Scripts

Faerry bundles generic sync scripts so the selected portable agents repo does
not need to carry its own script copies for app-driven actions.

Bundled scripts include:

- `link-agents.sh` / `link-agents.ps1`
- `sync-all-agents.sh` / `sync-all-agents.ps1`
- `sync-mcps.mjs`
- `sync-source-skills.mjs`
- `package-claude-skills.sh` / `package-claude-skills.ps1`
- `sync-claude-cowork-skills.mjs`

Repo-local scripts are still useful for CLI-first workflows. Faerry compares
repo-local copies with the bundled versions in the Advanced view.

## License

MIT
