#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import os from "node:os";

const scriptDir = path.dirname(new URL(import.meta.url).pathname);
const repoRoot = path.resolve(scriptDir, "..");
const aggregateSkillsDir = path.join(repoRoot, ".agents-manager", "runtime", "skills");
const repoSkillsDir = exists(aggregateSkillsDir) ? aggregateSkillsDir : path.join(repoRoot, "skills");
const syncStateFileName = ".portable-agents-sync.json";

const defaultAppRoots = [
  path.join(os.homedir(), "Library", "Application Support", "Claude"),
  path.join(os.homedir(), "Library", "Application Support", "Claude-3p"),
];
const legacyManagedSkillIds = new Set(["obsidian-vault"]);

const args = process.argv.slice(2);
const appRoots = [];
let dryRun = false;

for (let i = 0; i < args.length; i += 1) {
  const arg = args[i];
  if (arg === "--app-root") {
    appRoots.push(path.resolve(args[i + 1]));
    i += 1;
  } else if (arg === "--dry-run") {
    dryRun = true;
  } else if (arg === "--help" || arg === "-h") {
    usage();
    process.exit(0);
  } else {
    console.error(`Unknown option: ${arg}`);
    usage();
    process.exit(1);
  }
}

function usage() {
  console.log(`Usage: sync-claude-cowork-skills.mjs [options]

Copies repo-managed skills into Claude/Cowork skills-plugin workspaces.
This is for Claude Desktop, Claude Cowork, and Claude 3P, which do not consume
~/.agents/skills through stable symlinks in the same way as CLI agents.

Options:
  --app-root PATH  Claude application support root to sync. Can be repeated.
                   Defaults to Claude and Claude-3p on macOS.
  --dry-run        Print intended changes without copying.
  --help           Show this help.
`);
}

function exists(filePath) {
  return fs.existsSync(filePath);
}

function readJson(filePath, fallback) {
  if (!exists(filePath)) return structuredClone(fallback);
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function copyDirectory(source, target) {
  fs.mkdirSync(target, { recursive: true });
  fs.cpSync(source, target, {
    recursive: true,
    dereference: true,
    force: true,
    filter: (src) => path.basename(src) !== ".DS_Store",
  });
}

function removeDirectory(target) {
  fs.rmSync(target, { recursive: true, force: true });
}

function parseSkillMetadata(skillDir) {
  const skillPath = path.join(skillDir, "SKILL.md");
  const text = fs.readFileSync(skillPath, "utf8");
  const match = text.match(/^---\n([\s\S]*?)\n---/);
  const fallbackName = path.basename(skillDir);
  if (!match) return { name: fallbackName, description: "" };

  const frontmatter = match[1].split("\n");
  const metadata = {};
  for (let i = 0; i < frontmatter.length; i += 1) {
    const line = frontmatter[i];
    const separator = line.indexOf(":");
    if (separator === -1) continue;

    const key = line.slice(0, separator).trim();
    const rawValue = line.slice(separator + 1).trim();
    if (rawValue === ">") {
      const lines = [];
      i += 1;
      while (i < frontmatter.length && frontmatter[i].startsWith("  ")) {
        lines.push(frontmatter[i].trim());
        i += 1;
      }
      i -= 1;
      metadata[key] = lines.join(" ");
    } else {
      metadata[key] = rawValue.replace(/^["']|["']$/g, "");
    }
  }

  return {
    name: metadata.name || fallbackName,
    description: metadata.description || "",
  };
}

function repoSkills() {
  if (!exists(repoSkillsDir)) {
    throw new Error(`Missing repo skills directory: ${repoSkillsDir}`);
  }

  return fs
    .readdirSync(repoSkillsDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(repoSkillsDir, entry.name))
    .filter((skillDir) => exists(path.join(skillDir, "SKILL.md")))
    .sort();
}

function findSkillsPluginDirs(appRoot) {
  const pluginRoot = path.join(appRoot, "local-agent-mode-sessions", "skills-plugin");
  if (!exists(pluginRoot)) return [];

  const matches = [];
  for (const first of fs.readdirSync(pluginRoot, { withFileTypes: true })) {
    if (!first.isDirectory()) continue;
    const firstPath = path.join(pluginRoot, first.name);
    for (const second of fs.readdirSync(firstPath, { withFileTypes: true })) {
      if (!second.isDirectory()) continue;
      const workspaceRoot = path.join(firstPath, second.name);
      const skillsDir = path.join(workspaceRoot, "skills");
      if (exists(skillsDir)) {
        matches.push({ workspaceRoot, skillsDir });
      }
    }
  }
  return matches;
}

function syncStatePath(workspaceRoot) {
  return path.join(workspaceRoot, syncStateFileName);
}

function readSyncState(workspaceRoot) {
  return readJson(syncStatePath(workspaceRoot), { managedSkillIds: [] });
}

function writeSyncState(workspaceRoot, currentSkillIds) {
  const state = {
    updatedAt: new Date().toISOString(),
    managedSkillIds: [...currentSkillIds].sort(),
  };
  if (!dryRun) writeJson(syncStatePath(workspaceRoot), state);
}

function shouldPruneSkill(skill, staleManagedSkillIds) {
  const skillId = skill?.skillId || skill?.name;
  return (
    skillId &&
    (staleManagedSkillIds.has(skillId) || legacyManagedSkillIds.has(skillId))
  );
}

function pruneStaleSkillDirs(skillsDir, manifestSkills, currentFolderNames, staleManagedSkillIds) {
  const manifestById = new Map(manifestSkills.map((skill) => [skill.skillId || skill.name, skill]));
  const manifestByName = new Map(manifestSkills.map((skill) => [skill.name || skill.skillId, skill]));
  const pruned = [];

  for (const entry of fs.readdirSync(skillsDir, { withFileTypes: true })) {
    if (!entry.isDirectory() || currentFolderNames.has(entry.name)) {
      continue;
    }

    const manifestSkill = manifestById.get(entry.name) || manifestByName.get(entry.name);
    const staleManagedFolder = staleManagedSkillIds.has(entry.name) || legacyManagedSkillIds.has(entry.name);
    if (!(staleManagedFolder || (manifestSkill && shouldPruneSkill(manifestSkill, staleManagedSkillIds)))) {
      continue;
    }

    const target = path.join(skillsDir, entry.name);
    pruned.push(target);
    if (!dryRun) removeDirectory(target);
  }

  return pruned;
}

function updateManifest(workspaceRoot, metadataByName, staleManagedSkillIds) {
  const manifestPath = path.join(workspaceRoot, "manifest.json");
  const manifest = readJson(manifestPath, { skills: [] });
  const existingSkills = Array.isArray(manifest.skills) ? manifest.skills : [];
  const skills = existingSkills.filter((skill) => !shouldPruneSkill(skill, staleManagedSkillIds));
  const prunedCount = existingSkills.length - skills.length;
  const byId = new Map(skills.map((skill) => [skill.skillId, skill]));
  const updatedAt = new Date().toISOString();

  for (const [name, description] of metadataByName) {
    const existing = byId.get(name);
    if (existing) {
      existing.name = name;
      existing.description = description;
      existing.creatorType = existing.creatorType || "user";
      existing.syncManaged = false;
      existing.enabled = existing.enabled ?? true;
      existing.updatedAt = updatedAt;
    } else {
      skills.push({
        skillId: name,
        name,
        description,
        creatorType: "user",
        syncManaged: false,
        updatedAt,
        enabled: true,
      });
    }
  }

  manifest.lastUpdated = Date.now();
  manifest.skills = skills;
  if (!dryRun) writeJson(manifestPath, manifest);

  return prunedCount;
}

const targets = appRoots.length > 0 ? appRoots : defaultAppRoots;
const skills = repoSkills();
const metadataByName = new Map();
const currentFolderNames = new Set();
for (const skillDir of skills) {
  const metadata = parseSkillMetadata(skillDir);
  metadataByName.set(metadata.name, metadata.description);
  currentFolderNames.add(path.basename(skillDir));
}
const currentSkillIds = new Set(metadataByName.keys());

let workspaceCount = 0;
for (const appRoot of targets) {
  const workspaces = findSkillsPluginDirs(appRoot);
  if (workspaces.length === 0) {
    console.log(`No Claude/Cowork skills-plugin workspace found: ${appRoot}`);
    continue;
  }

  for (const { workspaceRoot, skillsDir } of workspaces) {
    workspaceCount += 1;
    console.log(`${dryRun ? "Would sync" : "Syncing"} ${skills.length} repo skills -> ${skillsDir}`);
    const syncState = readSyncState(workspaceRoot);
    const previouslyManagedSkillIds = new Set(syncState.managedSkillIds || []);
    const staleManagedSkillIds = new Set(
      [...previouslyManagedSkillIds].filter((skillId) => !currentSkillIds.has(skillId)),
    );
    const manifestPath = path.join(workspaceRoot, "manifest.json");
    const manifest = readJson(manifestPath, { skills: [] });
    const manifestSkills = Array.isArray(manifest.skills) ? manifest.skills : [];
    const prunedDirs = pruneStaleSkillDirs(skillsDir, manifestSkills, currentFolderNames, staleManagedSkillIds);
    for (const prunedDir of prunedDirs) {
      console.log(`${dryRun ? "Would remove" : "Removed"} stale repo-managed skill ${prunedDir}`);
    }
    if (!dryRun) {
      for (const skillDir of skills) {
        copyDirectory(skillDir, path.join(skillsDir, path.basename(skillDir)));
      }
    }
    const prunedManifestEntries = updateManifest(workspaceRoot, metadataByName, staleManagedSkillIds);
    if (prunedManifestEntries > 0) {
      console.log(`${dryRun ? "Would remove" : "Removed"} ${prunedManifestEntries} stale repo-managed manifest entr${prunedManifestEntries === 1 ? "y" : "ies"}`);
    }
    writeSyncState(workspaceRoot, currentSkillIds);
  }
}

console.log(`${dryRun ? "Checked" : "Synchronized"} ${workspaceCount} Claude/Cowork skill workspace(s).`);
