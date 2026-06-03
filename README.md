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

## Agent Workspace

This repository is initialized for agent-assisted development:

- `AGENTS.md` contains project-local instructions for coding agents.
- `DESIGN.md` defines the Faerry visual design system.
- `.agents/MEMORY.md` stores durable product and release decisions.
- `.agents/README.md` explains how to use the local agent memory folder.
- `faerry.json` keeps the workspace source configuration.

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

Run the full release process for the current platform:

```bash
npm run release
```

Create a portable package for the current platform:

```bash
npm run package:portable
```

## Release Process

Faerry releases are built on each target operating system. The same command is
used on macOS, Windows, and Linux:

```bash
npm run release
```

The release command runs:

- `npm run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- a Tauri release build for the current platform
- `npm run package:portable -- --no-build`
- a JSON release manifest with SHA-256 checksums

Portable artifacts are written to:

- macOS: `src-tauri/target/release/bundle/portable/faerry_<version>_macos-<arch>_portable.zip`
- Windows: `src-tauri/target/release/bundle/portable/faerry_<version>_windows-<arch>_portable.zip`
- Linux: `src-tauri/target/release/bundle/portable/faerry_<version>_linux-<arch>_portable.tar.gz`

Each portable artifact also gets a `.sha256` sidecar file. Release manifests are
written to `src-tauri/target/release/bundle/release/`.

Use `npm run package:portable -- --no-build` only when a release binary has
already been built and you want to rebuild the portable archive without running
checks or compiling again.

## GitHub Releases

GitHub Actions builds downloadable release assets for macOS, Windows, and Linux
through `.github/workflows/release.yml`.

Create a normal public release by pushing a version tag:

```bash
git tag v0.1.0-alpha.2
git push origin v0.1.0-alpha.2
```

The workflow builds every platform, uploads the portable archives as workflow
artifacts, then creates or updates the matching GitHub Release page. Users can
download the app from that release page under "Assets".

The workflow can also be started manually from the GitHub Actions UI. If no tag
is provided, it uses the version from `package.json`, for example
`v0.1.0-alpha.2`.

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
