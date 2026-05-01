# Portable Agents Manager

Tauri desktop GUI for managing a portable `.agents` repository.

The app keeps the repository as the source of truth:

- React/TypeScript renders the desktop UI.
- Rust performs read-only status detection and script planning.
- Bundled copies of the portable-agents scripts perform write/apply actions.
- No database and no local browser server in the packaged app.

## Install

```bash
npm install
```

Rust is required for Tauri development.

## Run

```bash
npm run dev
```

## Verify

```bash
npm run check
```

Build the packaged app with:

```bash
npm run build
```

## Apply Model

Before running any action, the GUI shows:

- command to run
- working directory
- likely affected paths
- whether backups may be created
- a diff preview where the app can compute one safely

Supported actions delegate to scripts bundled inside the app:

- `link-agents.sh` / `link-agents.ps1`
- `sync-all-agents.sh --dry-run-mcps` / PowerShell equivalent
- `sync-all-agents.sh --with-mcps` / PowerShell equivalent
- `sync-mcps.mjs --dry-run`
- `sync-mcps.mjs --target claude-code`
- `sync-mcps.mjs --target codex`
- `sync-mcps.mjs --target opencode`

The selected source-of-truth repo no longer needs to contain a `scripts/` folder for app-driven sync. Keeping repo-local scripts is still useful for CLI workflows.

Diff preview currently covers:

- symlink targets for global instructions, skills, commands, and shared `.agents` links
- GitHub Copilot CLI env snippet
- Codex MCP config text
- OpenCode MCP config JSON
- Claude Code MCP dry-run output through the existing script

## Repo Import

Use **Clone / Import** in the top bar to add another portable agents repo.

Supported sources:

- Git URLs, for example `https://github.com/example/agents.git`
- ZIP URLs, for example a GitHub `archive/refs/heads/main.zip`
- Local `.zip` files

The destination path must not already exist. After a successful import, the app switches to the imported repo as the selected source of truth.
