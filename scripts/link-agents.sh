#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)

HOME_DIR="${HOME:-}"
CODEX_ROOT="${CODEX_HOME:-$HOME_DIR/.codex}"
AGENTS_HOME="${HOME_DIR%/}/.agents"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_ROOT="${AGENTS_BACKUP_ROOT:-$HOME_DIR/.agents-backups/$TIMESTAMP}"

SKIP_CLAUDE=0
SKIP_OPENCODE=0
SKIP_CODEX=0
SKIP_COPILOT=0
ATLAS_DIR=""
CORTEX_DIR=""
BACKUP_COUNT=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

Links this repository into ~/.agents and the supported tool folders.
The repository stays the source of truth; installed locations are symlinks only.
If the repo already lives at ~/.agents, the shared links are skipped automatically.

Options:
  --repo-root PATH     Override the repository root.
  --home PATH          Override the target home directory.
  --codex-home PATH    Override the Codex home directory.
  --atlas-dir PATH     Optionally link Atlas vault AGENTS.md and CLAUDE.md.
  --cortex-dir PATH    Optionally link Cortex vault AGENTS.md and CLAUDE.md.
  --skip-claude        Do not link Claude Code folders.
  --skip-opencode      Do not link OpenCode folders.
  --skip-codex         Do not link Codex folders.
  --skip-copilot-env   Do not generate the GitHub Copilot CLI env snippet.
  --help               Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root)
      REPO_ROOT="$2"
      shift 2
      ;;
    --home)
      HOME_DIR="$2"
      CODEX_ROOT="${CODEX_HOME:-$HOME_DIR/.codex}"
      AGENTS_HOME="${HOME_DIR%/}/.agents"
      TIMESTAMP=$(date +%Y%m%d-%H%M%S)
      BACKUP_ROOT="${AGENTS_BACKUP_ROOT:-$HOME_DIR/.agents-backups/$TIMESTAMP}"
      shift 2
      ;;
    --codex-home)
      CODEX_ROOT="$2"
      shift 2
      ;;
    --atlas-dir)
      ATLAS_DIR="$2"
      shift 2
      ;;
    --cortex-dir)
      CORTEX_DIR="$2"
      shift 2
      ;;
    --skip-claude)
      SKIP_CLAUDE=1
      shift
      ;;
    --skip-opencode)
      SKIP_OPENCODE=1
      shift
      ;;
    --skip-codex)
      SKIP_CODEX=1
      shift
      ;;
    --skip-copilot-env)
      SKIP_COPILOT=1
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

if [[ -z "$HOME_DIR" ]]; then
  echo "HOME is not set." >&2
  exit 1
fi

if [[ ! -f "$REPO_ROOT/AGENTS.md" || ! -d "$REPO_ROOT/commands" || ! -d "$REPO_ROOT/skills" || ! -d "$REPO_ROOT/templates" ]]; then
  echo "Repo root does not match the expected .agents layout: $REPO_ROOT" >&2
  exit 1
fi

backup_target() {
  local target=$1
  local relative

  if [[ "$target" == "$HOME_DIR/"* ]]; then
    relative="${target#$HOME_DIR/}"
  elif [[ "$target" == "$HOME_DIR" ]]; then
    relative="home"
  else
    relative="absolute${target}"
  fi

  local destination="$BACKUP_ROOT/$relative"
  mkdir -p "$(dirname "$destination")"
  mv "$target" "$destination"
  BACKUP_COUNT=$((BACKUP_COUNT + 1))
  echo "  backed up $target -> $destination"
}

link_path() {
  local source=$1
  local target=$2
  local label=$3

  if [[ ! -e "$source" && ! -L "$source" ]]; then
    echo "Missing source for $label: $source" >&2
    exit 1
  fi

  mkdir -p "$(dirname "$target")"

  if [[ -L "$target" ]]; then
    local current
    current=$(readlink "$target")
    if [[ "$current" == "$source" ]]; then
      echo "  ok  $label"
      return
    fi
    backup_target "$target"
  elif [[ -e "$target" ]]; then
    backup_target "$target"
  fi

  ln -s "$source" "$target"
  echo "  link $label -> $target"
}

write_file() {
  local target=$1
  local content=$2

  mkdir -p "$(dirname "$target")"

  if [[ -e "$target" || -L "$target" ]]; then
    if [[ -f "$target" ]] && [[ "$(<"$target")" == "$content" ]]; then
      return
    fi
    backup_target "$target"
  fi

  printf '%s\n' "$content" > "$target"
  echo "  wrote $target"
}

install_shared_agents() {
  echo ""
  echo "Shared .agents"

  if [[ "${REPO_ROOT%/}" == "${AGENTS_HOME%/}" ]]; then
    echo "  repo already lives at $AGENTS_HOME"
    return
  fi

  mkdir -p "$AGENTS_HOME"
  link_path "$REPO_ROOT/AGENTS.md" "$AGENTS_HOME/AGENTS.md" "shared AGENTS.md"
  link_path "$REPO_ROOT/commands" "$AGENTS_HOME/commands" "shared commands"
  link_path "$REPO_ROOT/skills" "$AGENTS_HOME/skills" "shared skills"
  link_path "$REPO_ROOT/templates" "$AGENTS_HOME/templates" "shared templates"
}

install_claude() {
  [[ "$SKIP_CLAUDE" -eq 1 ]] && return
  echo ""
  echo "Claude Code"
  link_path "$AGENTS_HOME/AGENTS.md" "$HOME_DIR/.claude/CLAUDE.md" "Claude global context"
  link_path "$AGENTS_HOME/commands" "$HOME_DIR/.claude/commands" "Claude commands"
  link_path "$AGENTS_HOME/skills" "$HOME_DIR/.claude/skills" "Claude skills"
}

install_opencode() {
  [[ "$SKIP_OPENCODE" -eq 1 ]] && return
  echo ""
  echo "OpenCode"
  link_path "$AGENTS_HOME/AGENTS.md" "$HOME_DIR/.config/opencode/AGENTS.md" "OpenCode global context"
  link_path "$AGENTS_HOME/commands" "$HOME_DIR/.config/opencode/commands" "OpenCode commands"
  link_path "$AGENTS_HOME/skills" "$HOME_DIR/.config/opencode/skills" "OpenCode skills"
}

install_codex() {
  [[ "$SKIP_CODEX" -eq 1 ]] && return
  echo ""
  echo "Codex"
  link_path "$AGENTS_HOME/AGENTS.md" "$CODEX_ROOT/AGENTS.md" "Codex global context"
  link_path "$AGENTS_HOME/commands" "$CODEX_ROOT/prompts" "Codex prompts"
  link_path "$AGENTS_HOME/skills" "$CODEX_ROOT/skills" "Codex skills"
}

install_copilot_env() {
  [[ "$SKIP_COPILOT" -eq 1 ]] && return
  echo ""
  echo "GitHub Copilot CLI"

  local snippet_path="$HOME_DIR/.config/agents/github-copilot-cli.env.sh"
  local snippet_content
  snippet_content=$(cat <<EOF
export COPILOT_CUSTOM_INSTRUCTIONS_DIRS="$AGENTS_HOME"
export COPILOT_SKILLS_DIRS="$AGENTS_HOME/skills"
EOF
)

  write_file "$snippet_path" "$snippet_content"
  echo "  source $snippet_path in shells that run github-copilot-cli"
}

install_vault_link() {
  local label=$1
  local vault_dir=$2
  local template=$3

  [[ -z "$vault_dir" ]] && return

  echo ""
  echo "$label vault"
  link_path "$template" "$vault_dir/AGENTS.md" "$label AGENTS.md"
  link_path "$template" "$vault_dir/CLAUDE.md" "$label CLAUDE.md"
}

echo "agents repo linker"
echo "repo   $REPO_ROOT"
echo "target $HOME_DIR"

install_shared_agents
install_claude
install_opencode
install_codex
install_copilot_env
install_vault_link "Atlas" "$ATLAS_DIR" "$REPO_ROOT/templates/obsidian/AGENTS.atlas.md"
install_vault_link "Cortex" "$CORTEX_DIR" "$REPO_ROOT/templates/obsidian/AGENTS.cortex.md"

echo ""
if [[ "$BACKUP_COUNT" -gt 0 ]]; then
  echo "Backups: $BACKUP_ROOT"
else
  echo "Backups: none"
fi
