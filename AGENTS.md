# AGENTS.md - Faerry Agent Context

This file defines project-local guidance for coding agents working in the
Faerry repository.

## Product Context

Faerry is a Tauri desktop app for carrying agent configuration across tools.
It manages a local workspace as the source of truth for instructions, skills,
commands, designs, MCP servers, and optional external source repositories.

Primary audience:
- business users and consultants who need a simple UI
- technical users who still want predictable scripts and release artifacts
- teams that need portable agent setup across macOS, Windows, and Linux

Faerry should feel like a focused business tool, not an adesso-branded internal
utility and not a marketing landing page.

## Implementation Rules

- Keep React, Tauri, Rust, and vanilla CSS. Do not migrate UI framework unless
  explicitly requested.
- Keep functional changes scoped. Do not alter sync semantics, workspace
  schemas, or Tauri commands unless the task requires it.
- Prefer the existing local patterns in `src/main.tsx`, `src/styles.css`, and
  `src-tauri/src`.
- Preserve backward compatibility with legacy `sources.json` while writing new
  UI-managed source config to `faerry.json`.
- Treat `skills/`, `agents/`, `commands/`, `designs/`, `AGENTS.md`,
  `DESIGN.md`, `mcp/servers.json`, `faerry.json`, and legacy `sources.json` as
  valid workspace signals where the code supports them.
- Do not push to a remote unless the user explicitly asks.

## Design Rules

- Use `DESIGN.md` in this repository as the authoritative Faerry design system.
- Do not use adesso corporate styling, especially adesso blue `#006EC7` or
  Fira Sans Condensed branding.
- Preserve the current Ink + Seafoam / Transit OS direction.
- The UI should stay dense, calm, and business-friendly. Avoid redundant menus,
  repeated information panels, oversized hero sections, and decorative cards.
- Keep text readable at the Tauri minimum window size of `980 x 680`.

## Validation And Release

For normal code changes, run:

```bash
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
```

When work is done, always run the release process unless the user explicitly
asks not to or the environment makes it impossible:

```bash
npm run release
```

`npm run release` is the standard completion gate. It runs checks, Rust tests,
a Tauri release build, portable packaging, SHA-256 generation, and a release
manifest for the current platform.

## Important Files

- `src/main.tsx` - React UI and app state wiring.
- `src/styles.css` - Faerry design tokens and UI styling.
- `src-tauri/src` - Rust backend commands and workspace logic.
- `scripts/release.mjs` - standard release process.
- `scripts/package-portable.mjs` - platform-specific portable artifact packer.
- `faerry.json` - source repository configuration for this workspace.
- `.agents/MEMORY.md` - durable project decisions for future agents.
