# Faerry

Faerry is a desktop app for carrying agent configuration across tools. It keeps
one workspace as the source of truth for instructions, skills, MCP servers, and
optional source repositories, then links or syncs that setup into tools such as
Claude Code, Codex, OpenCode, GitHub Copilot CLI, and Claude Desktop skill
packages.

Faerry combines the practical idea of a ferry with a small hint of faery-like
magic. The "ae" spelling is a personal signature, while the product remains a
general-purpose portable agents manager.

## Status

Faerry is early software. The app currently focuses on local desktop workflows
for Faerry-compatible workspaces:

- inspect workspace health, skills, designs, MCP registries, and source repos
- preview link and MCP changes before applying them
- link `~/.agents` into supported CLI agent tools
- manage local or Git sources from the UI through `faerry.json`
- package Claude Desktop skill ZIPs
- import workspaces from Git URLs, ZIP URLs, or local ZIP files

## Faerry Workspace Contract

Faerry accepts a folder as a workspace when it contains at least one useful
agent-management signal:

```text
.
├── faerry.json        # preferred config for sources
├── AGENTS.md          # optional global instructions
├── skills/            # optional local skills
├── agents/            # optional agent definitions
├── commands/          # optional commands
├── designs/ or DESIGN.md
└── mcp/servers.json   # optional MCP registry
```

`skills/` is no longer required. A source-only workspace with just `faerry.json`
is valid. Legacy `sources.json` files are still read, but new UI edits write
`faerry.json`.

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

Faerry bundles generic sync scripts so the selected workspace does
not need to carry its own script copies for app-driven actions.

Bundled scripts include:

- `link-agents.sh` / `link-agents.ps1`
- `sync-all-agents.sh` / `sync-all-agents.ps1`
- `sync-mcps.mjs`
- `sync-source-skills.mjs`
- `package-claude-skills.sh` / `package-claude-skills.ps1`
- `sync-claude-cowork-skills.mjs`

Repo-local scripts are still useful for CLI-first workflows. Faerry compares
repo-local copies with the bundled versions in Settings.

## License

MIT
