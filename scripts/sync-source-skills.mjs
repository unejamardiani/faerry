#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const args = process.argv.slice(2);
let repoRoot = process.env.PORTABLE_AGENTS_REPO_ROOT || path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");

for (let i = 0; i < args.length; i += 1) {
  const arg = args[i];
  if (arg === "--repo-root") {
    repoRoot = path.resolve(args[i + 1]);
    i += 1;
  } else if (arg === "--help" || arg === "-h") {
    usage();
    process.exit(0);
  } else {
    console.error(`Unknown option: ${arg}`);
    usage();
    process.exit(1);
  }
}

const sourcesPath = path.join(repoRoot, "sources.json");
const localSkillsDir = path.join(repoRoot, "skills");
const cacheRoot = path.join(repoRoot, ".agents-manager", "source-cache");
const runtimeRoot = path.join(repoRoot, ".agents-manager", "runtime");
const aggregateSkillsDir = path.join(runtimeRoot, "skills");

function usage() {
  console.log(`Usage: sync-source-skills.mjs [options]

Builds a generated aggregate skills directory from repo-local skills and enabled
skill sources in sources.json.

Options:
  --repo-root PATH  Portable agents repo root. Defaults to parent of scripts/.
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

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function listDirs(dir) {
  if (!exists(dir)) return [];
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() || entry.isSymbolicLink())
    .map((entry) => entry.name)
    .sort();
}

function parseFrontmatter(filePath) {
  if (!exists(filePath)) return {};
  const text = fs.readFileSync(filePath, "utf8");
  const match = text.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return {};
  const metadata = {};
  const lines = match[1].split("\n");
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const separator = line.indexOf(":");
    if (separator === -1) continue;
    const key = line.slice(0, separator).trim();
    const rawValue = line.slice(separator + 1).trim();
    if (rawValue === ">" || rawValue === "|") {
      const values = [];
      i += 1;
      while (i < lines.length && /^\s+/.test(lines[i])) {
        values.push(lines[i].trim());
        i += 1;
      }
      i -= 1;
      metadata[key] = values.join(" ");
    } else {
      metadata[key] = rawValue.replace(/^["']|["']$/g, "");
    }
  }
  return metadata;
}

function isUrl(value) {
  return /^https?:\/\//i.test(value) || /^git@/i.test(value);
}

function resolveHome(value) {
  if (value === "~") return process.env.HOME || value;
  if (value.startsWith("~/")) return path.join(process.env.HOME || "", value.slice(2));
  return value;
}

function resolveLocalPath(value, root = repoRoot) {
  const expanded = resolveHome(value.trim());
  return path.isAbsolute(expanded) ? expanded : path.resolve(root, expanded);
}

function sourceRef(source) {
  return source.ref || source.branch || null;
}

function parseGitHubUrl(url, explicitRef) {
  const clean = url.split(/[?#]/)[0].replace(/\/$/, "");
  const match = clean.match(/^https?:\/\/(?:www\.)?github\.com\/([^/]+)\/([^/]+)(?:\/(tree|blob)\/([^/]+)(?:\/(.*))?)?$/i);
  if (!match) return null;

  const [, owner, repoNameRaw, kind, urlRef, subpathRaw] = match;
  const repoName = repoNameRaw.replace(/\.git$/, "");
  const cloneUrl = `https://github.com/${owner}/${repoName}.git`;
  const gitRef = explicitRef || urlRef || null;
  let subpath = subpathRaw || "";
  if (kind === "blob" && subpath.endsWith("/SKILL.md")) {
    subpath = subpath.slice(0, -"/SKILL.md".length);
  } else if (kind === "blob" && subpath === "SKILL.md") {
    subpath = "";
  }
  return { cloneUrl, gitRef, subpath };
}

function gitSourceSpec(source) {
  const explicitRef = sourceRef(source);
  const parsed = parseGitHubUrl(source.url, explicitRef);
  if (parsed) return parsed;
  return { cloneUrl: source.url, gitRef: explicitRef, subpath: "" };
}

function stableHash(value) {
  let hash = 0xcbf29ce484222325n;
  for (const byte of Buffer.from(value)) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * 0x100000001b3n);
  }
  return hash.toString(16).padStart(16, "0");
}

function slugify(value) {
  const slug = value.toLowerCase().replace(/[^a-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "");
  return slug || "source";
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { encoding: "utf8", ...options });
  if (result.status !== 0) {
    const detail = (result.stderr || result.stdout || "").trim();
    throw new Error(`${command} ${args.join(" ")} failed${detail ? `: ${detail}` : ""}`);
  }
  return result;
}

function ensureGitSource(source, name) {
  const spec = gitSourceSpec(source);
  const key = `${spec.cloneUrl}|${spec.gitRef || ""}|${spec.subpath || ""}`;
  const checkout = path.join(cacheRoot, `${slugify(name)}-${stableHash(key)}`);
  if (exists(path.join(checkout, ".git"))) {
    if (source.refresh) {
      const fetchArgs = ["-C", checkout, "fetch", "--depth", "1"];
      if (spec.gitRef) fetchArgs.push("origin", spec.gitRef);
      run("git", fetchArgs);
    }
    if (spec.gitRef) run("git", ["-C", checkout, "checkout", spec.gitRef]);
  } else {
    ensureDir(cacheRoot);
    const cloneArgs = ["clone", "--depth", "1"];
    if (spec.gitRef) cloneArgs.push("--branch", spec.gitRef);
    cloneArgs.push(spec.cloneUrl, checkout);
    run("git", cloneArgs);
  }
  return spec.subpath ? path.join(checkout, spec.subpath) : checkout;
}

function sourceRoot(source, name) {
  if (source.url) return ensureGitSource(source, name);
  if (source.path) return resolveLocalPath(source.path);
  throw new Error(`Source ${name} must define path or url.`);
}

function globMatch(pattern, value) {
  if (!pattern) return false;
  const escaped = pattern
    .replace(/[.+^${}()|[\]\\]/g, "\\$&")
    .replace(/\*/g, ".*")
    .replace(/\?/g, ".");
  return new RegExp(`^${escaped}$`).test(value);
}

function allowed(candidates, includes = [], excludes = []) {
  const included = includes.length === 0 || includes.some((pattern) => candidates.some((candidate) => globMatch(pattern, candidate)));
  if (!included) return false;
  return !excludes.some((pattern) => candidates.some((candidate) => globMatch(pattern, candidate)));
}

function hasSkill(dir) {
  return exists(path.join(dir, "SKILL.md"));
}

function resolveSkillDirs(root, source) {
  if (Array.isArray(source.skillPaths) && source.skillPaths.length > 0) {
    return source.skillPaths.map((entry) => resolveLocalPath(entry, root)).filter(hasSkill);
  }
  if (source.skillsPath) {
    const target = resolveLocalPath(source.skillsPath, root);
    if (hasSkill(target)) return [target];
    return listDirs(target).map((entry) => path.join(target, entry)).filter(hasSkill);
  }
  if (hasSkill(root)) return [root];
  const conventional = path.join(root, "skills");
  if (exists(conventional)) return listDirs(conventional).map((entry) => path.join(conventional, entry)).filter(hasSkill);
  return listDirs(root).map((entry) => path.join(root, entry)).filter(hasSkill);
}

function symlinkEntry(source, target) {
  fs.symlinkSync(source, target, fs.statSync(source).isDirectory() ? "dir" : "file");
}

function uniqueName(baseName, sourceName, usedNames) {
  if (!usedNames.has(baseName)) return baseName;
  const prefixed = `${slugify(sourceName)}-${baseName}`;
  if (!usedNames.has(prefixed)) return prefixed;
  let index = 2;
  while (usedNames.has(`${prefixed}-${index}`)) index += 1;
  return `${prefixed}-${index}`;
}

function addSkillLink(tmpDir, usedNames, skillDir, sourceName, options = {}) {
  const folderName = path.basename(skillDir);
  const metadata = parseFrontmatter(path.join(skillDir, "SKILL.md"));
  const displayName = metadata.name || folderName;
  const relative = path.relative(options.root || path.dirname(skillDir), skillDir).replaceAll(path.sep, "/");
  if (!allowed([folderName, displayName, relative], options.include || [], options.exclude || [])) return false;

  const targetName = uniqueName(folderName, sourceName, usedNames);
  usedNames.add(targetName);
  symlinkEntry(skillDir, path.join(tmpDir, targetName));
  return true;
}

function buildAggregate() {
  if (!exists(localSkillsDir)) {
    throw new Error(`Missing local skills directory: ${localSkillsDir}`);
  }

  const sources = readJson(sourcesPath, { sources: [] }).sources || [];
  const tmpDir = path.join(runtimeRoot, `skills.tmp-${process.pid}`);
  fs.rmSync(tmpDir, { recursive: true, force: true });
  ensureDir(tmpDir);

  const usedNames = new Set();
  for (const entry of listDirs(localSkillsDir)) {
    const source = path.join(localSkillsDir, entry);
    usedNames.add(entry);
    symlinkEntry(source, path.join(tmpDir, entry));
  }

  let linkedSourceSkills = 0;
  for (let index = 0; index < sources.length; index += 1) {
    const source = sources[index];
    if (source.enabled === false || source.skills === false) continue;
    const name = source.name || `source-${index + 1}`;
    const root = sourceRoot(source, name);
    const skillDirs = resolveSkillDirs(root, source);
    for (const skillDir of skillDirs) {
      if (addSkillLink(tmpDir, usedNames, skillDir, name, {
        root,
        include: source.includeSkills || [],
        exclude: source.excludeSkills || [],
      })) {
        linkedSourceSkills += 1;
      }
    }
  }

  fs.rmSync(aggregateSkillsDir, { recursive: true, force: true });
  ensureDir(runtimeRoot);
  fs.renameSync(tmpDir, aggregateSkillsDir);
  return linkedSourceSkills;
}

const count = buildAggregate();
console.log(`Source skills aggregated at ${aggregateSkillsDir}`);
console.log(`Source skills linked: ${count}`);
