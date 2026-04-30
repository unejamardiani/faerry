#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
SKILLS_DIR="$REPO_ROOT/skills"
OUTPUT_DIR="$REPO_ROOT/dist/claude-desktop/skills"

if ! command -v zip >/dev/null 2>&1; then
  echo "zip is required." >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"

declare -a TARGETS=()
package_is_current() {
  local package_name="$1"
  local skill_name
  for skill_name in "${TARGETS[@]}"; do
    if [[ "$package_name" == "$skill_name.zip" ]]; then
      return 0
    fi
  done
  return 1
}

if [[ $# -gt 0 ]]; then
  TARGETS=("$@")
else
  while IFS= read -r skill_dir; do
    TARGETS+=("$(basename "$skill_dir")")
  done < <(find "$SKILLS_DIR" -mindepth 1 -maxdepth 1 -type d -exec test -f '{}/SKILL.md' ';' -print | sort)

  while IFS= read -r package_path; do
    package_name=$(basename "$package_path")
    if ! package_is_current "$package_name"; then
      rm -f "$package_path"
      echo "Removed stale package $package_path"
    fi
  done < <(find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -type f -name '*.zip' -print | sort)
fi

for skill_name in "${TARGETS[@]}"; do
  skill_path="$SKILLS_DIR/$skill_name"
  if [[ ! -f "$skill_path/SKILL.md" ]]; then
    echo "Skipping $skill_name: missing SKILL.md"
    continue
  fi

  temp_dir=$(mktemp -d)
  trap 'rm -rf "$temp_dir"' EXIT
  cp -R "$skill_path" "$temp_dir/$skill_name"
  find "$temp_dir" -name '.DS_Store' -delete

  output_path="$OUTPUT_DIR/$skill_name.zip"
  rm -f "$output_path"
  (
    cd "$temp_dir"
    zip -rq "$output_path" "$skill_name"
  )
  rm -rf "$temp_dir"
  trap - EXIT

  echo "Created $output_path"
done
