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

Git imports support an optional branch/tag and shallow clone mode. The destination is auto-derived from common Git/ZIP source names but remains editable before running.

## Resource Sources

The app can load skills and commands from additional local source checkouts at startup/refresh time. Add a `sources.json` file to the selected repo root:

```json
{
  "sources": [
    {
      "name": "team-shared",
      "path": "../team-agents",
      "enabled": true,
      "skills": true,
      "commands": true
    },
    {
      "name": "vendor-skills",
      "path": "~/agent-sources/vendor",
      "skillsPath": "skills",
      "commands": false
    },
    {
      "name": "github-skills-folder",
      "url": "https://github.com/Leonxlnx/taste-skill/tree/main/skills",
      "skills": true,
      "commands": false,
      "includeSkills": [
        "taste-*",
        "planner"
      ]
    },
    {
      "name": "github-selected-skills",
      "url": "https://github.com/example/agents.git",
      "ref": "main",
      "skills": true,
      "commands": false,
      "skillPaths": [
        "skills/first-skill",
        "skills/second-skill"
      ]
    },
    {
      "name": "google-design-md-examples",
      "url": "https://github.com/google-labs-code/design.md.git",
      "ref": "main",
      "skills": false,
      "commands": false,
      "designs": true,
      "designsPath": "examples",
      "includeDesigns": [
        "*"
      ]
    }
  ]
}
```

Local `path` entries may be absolute, `~/...`, or relative to the selected repo. Git `url` entries are cloned into `.agents-manager/source-cache/` under the selected repo and loaded from that cache. GitHub `tree/...` links can point at a repo root, a `skills/` folder, or a single skill folder containing `SKILL.md`; GitHub `blob/.../SKILL.md` links are treated as that single skill folder.

By default the app reads `skills/` and `commands/` under each source. Set `designs: true` to also load DESIGN.md files. Use `skillsPath`, `commandsPath`, or `designsPath` when a source uses a different folder layout, or `skillPaths` / `commandPaths` / `designPaths` to list specific entries from one source. Set `ref` or `branch` for Git branches/tags. Set `refresh: true` when the app should fetch the Git source on every load; otherwise it clones once and reuses the cache.

For folders with many skills, use `includeSkills` and `excludeSkills` as simple glob lists. Patterns are matched against the skill folder name, the skill display name from frontmatter, and the path relative to the source root. If `includeSkills` is omitted or empty, all discovered skills are included. `excludeSkills` is applied after includes. Commands have the same optional `includeCommands` and `excludeCommands` filters.

Design.md support is read-only in the manager. The app discovers root-level `DESIGN.md`, `designs/DESIGN.md`, `designs/<name>/DESIGN.md`, and configured design paths. `includeDesigns` and `excludeDesigns` use the same simple glob matching as skills.

The local selected repo wins on name conflicts. External skills and commands are shown with `source-only` install status because the current sync scripts still link only the selected repo's own `skills/` and `commands/` directories.

## Inspection Views

The app includes read-only inspection views for:

- runtime diagnostics, dependency availability, bundled script checksums, and repo-vs-bundled script comparison
- repo validation and safety warnings
- skill, command, and MCP server detail panes
- standalone diff previews
- recent logs and parsed backup paths
- read-only profiles

MCP auth status is best-effort. Where CLIs do not expose a stable status command, the UI shows the manual command to run after sync.

## Selective Sync

Selective sync planning is available in the **Editor** view. MCP sync can be planned per supported target tool. Link sync is still limited by the bundled scripts and currently runs the whole link script when globals, skills, or commands are selected.
