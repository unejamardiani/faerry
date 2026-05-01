#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)

WITH_CLAUDE_PACKAGES=1
WITH_CLAUDE_EXTENSION=1
WITH_COWORK_LIVE=0
DRY_RUN_COWORK=0
WITH_MCPS=0
DRY_RUN_MCPS=0

declare -a LINK_ARGS=()

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

One-command synchronization for the portable agents repository.

Default behavior:
  1. Link ~/.agents to this repository.
  2. Link Claude Code, OpenCode, Codex, and Copilot CLI to ~/.agents.
  3. Build Claude Desktop skill ZIPs from repo skills.
  4. Build the Claude Desktop MCPB extension from repo commands.

Claude Desktop / Cowork / Claude 3P live app data is not modified by default.
Use --with-cowork-live when you explicitly want to copy repo skills into
existing Claude/Cowork skills-plugin workspaces.

Options:
  --repo-root PATH            Override repository root.
  --home PATH                 Pass through to link-agents.sh.
  --codex-home PATH           Pass through to link-agents.sh.
  --atlas-dir PATH            Pass through to link-agents.sh.
  --cortex-dir PATH           Pass through to link-agents.sh.
  --skip-claude               Pass through to link-agents.sh.
  --skip-opencode             Pass through to link-agents.sh.
  --skip-codex                Pass through to link-agents.sh.
  --skip-copilot-env          Pass through to link-agents.sh.
  --skip-claude-packages      Do not build Claude Desktop skill ZIPs.
  --skip-claude-extension     Do not build the Claude Desktop MCPB extension.
  --with-claude-extension     Deprecated no-op; the extension is built by default.
  --with-cowork-live          Copy repo skills into Claude/Cowork/Claude-3P live workspaces.
  --dry-run-cowork            Show Cowork live sync targets without copying.
  --with-mcps                 Sync repo-managed MCP servers into Claude Code, Codex, and OpenCode.
  --dry-run-mcps              Show MCP sync changes without writing.
  --help                      Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root)
      REPO_ROOT="$2"
      LINK_ARGS+=("--repo-root" "$2")
      shift 2
      ;;
    --home|--codex-home|--atlas-dir|--cortex-dir)
      LINK_ARGS+=("$1" "$2")
      shift 2
      ;;
    --skip-claude|--skip-opencode|--skip-codex|--skip-copilot-env)
      LINK_ARGS+=("$1")
      shift
      ;;
    --skip-claude-packages)
      WITH_CLAUDE_PACKAGES=0
      shift
      ;;
    --skip-claude-extension)
      WITH_CLAUDE_EXTENSION=0
      shift
      ;;
    --with-claude-extension)
      WITH_CLAUDE_EXTENSION=1
      shift
      ;;
    --with-cowork-live)
      WITH_COWORK_LIVE=1
      shift
      ;;
    --dry-run-cowork)
      DRY_RUN_COWORK=1
      WITH_COWORK_LIVE=1
      shift
      ;;
    --with-mcps)
      WITH_MCPS=1
      shift
      ;;
    --dry-run-mcps)
      DRY_RUN_MCPS=1
      WITH_MCPS=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

SCRIPT_DIR="$REPO_ROOT/scripts"

echo "portable agents sync"
echo "repo $REPO_ROOT"

if [[ "${#LINK_ARGS[@]}" -gt 0 ]]; then
  "$SCRIPT_DIR/link-agents.sh" "${LINK_ARGS[@]}"
else
  "$SCRIPT_DIR/link-agents.sh"
fi

if [[ "$WITH_CLAUDE_PACKAGES" -eq 1 ]]; then
  echo ""
  echo "Claude Desktop skill packages"
  "$SCRIPT_DIR/package-claude-skills.sh"
fi

if [[ "$WITH_CLAUDE_EXTENSION" -eq 1 ]]; then
  echo ""
  echo "Claude Desktop extension"
  "$SCRIPT_DIR/package-claude-extension.sh"
fi

if [[ "$WITH_COWORK_LIVE" -eq 1 ]]; then
  echo ""
  echo "Claude/Cowork live skill workspaces"
  if [[ "$DRY_RUN_COWORK" -eq 1 ]]; then
    node "$SCRIPT_DIR/sync-claude-cowork-skills.mjs" --dry-run
  else
    node "$SCRIPT_DIR/sync-claude-cowork-skills.mjs"
  fi
fi

if [[ "$WITH_MCPS" -eq 1 ]]; then
  echo ""
  echo "MCP servers"
  if [[ "$DRY_RUN_MCPS" -eq 1 ]]; then
    node "$SCRIPT_DIR/sync-mcps.mjs" --dry-run
  else
    node "$SCRIPT_DIR/sync-mcps.mjs"
  fi
fi

echo ""
echo "Done."
