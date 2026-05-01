#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const scriptDir = path.dirname(new URL(import.meta.url).pathname);
const repoRoot = path.resolve(scriptDir, "..");
const defaultRegistryPath = path.join(repoRoot, "mcp", "servers.json");

const managedStart = "# BEGIN portable-agents managed MCP servers";
const managedEnd = "# END portable-agents managed MCP servers";

const options = {
  registryPath: defaultRegistryPath,
  targets: new Set(["claudeCode", "codex", "opencode"]),
  homeDir: os.homedir(),
  codexHome: process.env.CODEX_HOME || path.join(os.homedir(), ".codex"),
  opencodeConfig: path.join(os.homedir(), ".config", "opencode", "opencode.json"),
  dryRun: false,
};

for (let i = 2; i < process.argv.length; i += 1) {
  const arg = process.argv[i];
  if (arg === "--registry") {
    options.registryPath = path.resolve(process.argv[++i]);
  } else if (arg === "--target") {
    options.targets = parseTargets(process.argv[++i]);
  } else if (arg === "--home") {
    options.homeDir = path.resolve(process.argv[++i]);
    options.codexHome = process.env.CODEX_HOME || path.join(options.homeDir, ".codex");
    options.opencodeConfig = path.join(options.homeDir, ".config", "opencode", "opencode.json");
  } else if (arg === "--codex-home") {
    options.codexHome = path.resolve(process.argv[++i]);
  } else if (arg === "--opencode-config") {
    options.opencodeConfig = path.resolve(process.argv[++i]);
  } else if (arg === "--dry-run") {
    options.dryRun = true;
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
  console.log(`Usage: sync-mcps.mjs [options]

Synchronizes repo-managed MCP servers into Claude Code, Codex, and OpenCode.
The repo registry is the source of truth for MCP server names it contains.
Unrelated MCP servers and unrelated tool settings are preserved.

Options:
  --registry PATH          MCP registry JSON. Defaults to mcp/servers.json.
  --target TARGETS         all, claude, claude-code, codex, or opencode.
                           Comma-separated values are accepted.
  --home PATH              Target home directory. Defaults to $HOME.
  --codex-home PATH        Codex home. Defaults to $CODEX_HOME or ~/.codex.
  --opencode-config PATH   OpenCode config path.
  --dry-run                Print intended changes without writing.
  --help                   Show this help.
`);
}

function parseTargets(rawValue) {
  const values = rawValue.split(",").map((value) => value.trim()).filter(Boolean);
  const targets = new Set();
  for (const value of values) {
    if (value === "all") {
      targets.add("claudeCode");
      targets.add("codex");
      targets.add("opencode");
    } else if (value === "claude" || value === "claude-code" || value === "claudeCode") {
      targets.add("claudeCode");
    } else if (value === "codex") {
      targets.add("codex");
    } else if (value === "opencode") {
      targets.add("opencode");
    } else {
      throw new Error(`Unknown target: ${value}`);
    }
  }
  return targets;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function ensureParent(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function timestamp() {
  return new Date().toISOString().replace(/[-:]/g, "").replace(/\..+/, "").replace("T", "-");
}

function backupFile(filePath) {
  if (!fs.existsSync(filePath) || options.dryRun) return;
  fs.copyFileSync(filePath, `${filePath}.bak-${timestamp()}`);
}

function writeFileIfChanged(filePath, content) {
  const current = fs.existsSync(filePath) ? fs.readFileSync(filePath, "utf8") : "";
  if (current === content) {
    console.log(`  ok  ${filePath}`);
    return;
  }
  if (options.dryRun) {
    console.log(`  would write ${filePath}`);
    return;
  }
  ensureParent(filePath);
  backupFile(filePath);
  fs.writeFileSync(filePath, content);
  console.log(`  wrote ${filePath}`);
}

function registryServers() {
  const registry = readJson(options.registryPath);
  return Object.entries(registry.servers || {}).map(([name, server]) => [name, normalizeServer(name, server)]);
}

function normalizeServer(name, server) {
  const type = server.type || (server.url ? "remote" : "local");
  const enabled = server.enabled !== false;
  if (type === "remote" && !server.url) {
    throw new Error(`MCP server ${name} is remote but has no url.`);
  }
  if (type === "local" && !server.command) {
    throw new Error(`MCP server ${name} is local but has no command.`);
  }
  return {
    ...server,
    type,
    transport: server.transport || (type === "remote" ? "http" : "stdio"),
    enabled,
    args: server.args || [],
    targets: server.targets || {},
  };
}

function targetEnabled(server, target) {
  return server.targets[target] !== false;
}

function jsonForClaude(server) {
  if (server.type === "remote") {
    return {
      type: "http",
      url: server.url,
      ...(server.headers ? { headers: server.headers } : {}),
    };
  }

  return {
    type: "stdio",
    command: server.command,
    args: server.args || [],
    ...(server.environment ? { env: server.environment } : {}),
  };
}

function run(command, args, label) {
  if (options.dryRun) {
    console.log(`  would run ${command} ${args.map(shellArg).join(" ")}`);
    return { status: 0 };
  }

  const result = spawnSync(command, args, { stdio: "pipe", encoding: "utf8" });
  if (result.error?.code === "ENOENT") {
    console.warn(`  skip ${label}: ${command} not found`);
    return result;
  }
  if (result.status !== 0) {
    const output = [result.stdout, result.stderr].filter(Boolean).join("\n").trim();
    console.warn(`  warning: ${label} failed${output ? `\n${indent(output)}` : ""}`);
  }
  return result;
}

function shellArg(value) {
  if (/^[A-Za-z0-9_./:=@-]+$/.test(value)) return value;
  return JSON.stringify(value);
}

function indent(text) {
  return text.split("\n").map((line) => `    ${line}`).join("\n");
}

function syncClaudeCode(servers) {
  console.log("");
  console.log("Claude Code MCP");
  for (const [name, server] of servers) {
    if (!targetEnabled(server, "claudeCode")) continue;

    run("claude", ["mcp", "remove", "--scope", "user", name], `remove Claude Code MCP ${name}`);
    const config = JSON.stringify(jsonForClaude(server));
    run("claude", ["mcp", "add-json", "--scope", "user", name, config], `add Claude Code MCP ${name}`);
    console.log(`  managed ${name}`);
  }
}

function tomlKey(name) {
  return /^[A-Za-z0-9_-]+$/.test(name) ? name : JSON.stringify(name);
}

function tomlString(value) {
  return JSON.stringify(String(value));
}

function tomlBool(value) {
  return value ? "true" : "false";
}

function tomlArray(values) {
  return `[${values.map(tomlString).join(", ")}]`;
}

function tomlInlineObject(values) {
  const entries = Object.entries(values || {});
  return `{ ${entries.map(([key, value]) => `${tomlString(key)} = ${tomlString(value)}`).join(", ")} }`;
}

function codexTomlBlock(servers) {
  const lines = [managedStart, "# Source: ~/.agents/mcp/servers.json"];
  for (const [name, server] of servers) {
    if (!targetEnabled(server, "codex")) continue;

    lines.push("", `[mcp_servers.${tomlKey(name)}]`);
    if (server.type === "remote") {
      lines.push(`url = ${tomlString(server.url)}`);
      if (server.headers) lines.push(`http_headers = ${tomlInlineObject(server.headers)}`);
    } else {
      lines.push(`command = ${tomlString(server.command)}`);
      if (server.args?.length) lines.push(`args = ${tomlArray(server.args)}`);
      if (server.environment) lines.push(`env = ${tomlInlineObject(server.environment)}`);
    }
    lines.push(`enabled = ${tomlBool(server.enabled)}`);
  }
  lines.push("", managedEnd, "");
  return lines.join("\n");
}

function removeManagedToml(text, serverNames) {
  const markerPattern = new RegExp(`${escapeRegex(managedStart)}[\\s\\S]*?${escapeRegex(managedEnd)}\\n?`, "g");
  let next = text.replace(markerPattern, "").trimEnd();
  next = removeCodexTables(next, serverNames).trimEnd();
  return next ? `${next}\n\n` : "";
}

function removeCodexTables(text, serverNames) {
  if (!text) return text;
  const lines = text.split("\n");
  const output = [];
  let dropping = false;

  for (const line of lines) {
    const header = line.match(/^\s*\[([^\]]+)]\s*$/);
    if (header) {
      const table = header[1].trim();
      dropping = isManagedCodexTable(table, serverNames);
    }
    if (!dropping) output.push(line);
  }

  return output.join("\n");
}

function isManagedCodexTable(table, serverNames) {
  for (const name of serverNames) {
    const bare = `mcp_servers.${name}`;
    const quoted = `mcp_servers.${JSON.stringify(name)}`;
    if (table === bare || table.startsWith(`${bare}.`) || table === quoted || table.startsWith(`${quoted}.`)) {
      return true;
    }
  }
  return false;
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function syncCodex(servers) {
  console.log("");
  console.log("Codex MCP");
  const configPath = path.join(options.codexHome, "config.toml");
  const current = fs.existsSync(configPath) ? fs.readFileSync(configPath, "utf8") : "";
  const managedNames = servers.filter(([, server]) => targetEnabled(server, "codex")).map(([name]) => name);
  const next = `${removeManagedToml(current, managedNames)}${codexTomlBlock(servers)}`;
  writeFileIfChanged(configPath, next);
}

function stripJsonComments(text) {
  let output = "";
  let inString = false;
  let stringQuote = "";
  let escaped = false;

  for (let i = 0; i < text.length; i += 1) {
    const char = text[i];
    const next = text[i + 1];

    if (inString) {
      output += char;
      if (escaped) {
        escaped = false;
      } else if (char === "\\") {
        escaped = true;
      } else if (char === stringQuote) {
        inString = false;
      }
      continue;
    }

    if (char === '"' || char === "'") {
      inString = true;
      stringQuote = char;
      output += char;
      continue;
    }

    if (char === "/" && next === "/") {
      while (i < text.length && text[i] !== "\n") i += 1;
      output += "\n";
      continue;
    }

    if (char === "/" && next === "*") {
      i += 2;
      while (i < text.length && !(text[i] === "*" && text[i + 1] === "/")) i += 1;
      i += 1;
      continue;
    }

    output += char;
  }

  return output.replace(/,\s*([}\]])/g, "$1");
}

function readJsonConfig(filePath) {
  if (!fs.existsSync(filePath)) return {};
  const text = fs.readFileSync(filePath, "utf8");
  if (!text.trim()) return {};
  return JSON.parse(stripJsonComments(text));
}

function opencodeServer(server) {
  if (server.type === "remote") {
    return {
      type: "remote",
      url: server.url,
      enabled: server.enabled,
      ...(server.headers ? { headers: server.headers } : {}),
    };
  }

  return {
    type: "local",
    command: [server.command, ...(server.args || [])],
    enabled: server.enabled,
    ...(server.environment ? { environment: server.environment } : {}),
  };
}

function syncOpenCode(servers) {
  console.log("");
  console.log("OpenCode MCP");
  const config = readJsonConfig(options.opencodeConfig);
  config.$schema = config.$schema || "https://opencode.ai/config.json";
  config.mcp = config.mcp && typeof config.mcp === "object" ? config.mcp : {};

  for (const [name, server] of servers) {
    if (!targetEnabled(server, "opencode")) continue;
    config.mcp[name] = opencodeServer(server);
    console.log(`  managed ${name}`);
  }

  writeFileIfChanged(options.opencodeConfig, `${JSON.stringify(config, null, 2)}\n`);
}

const servers = registryServers();

console.log("MCP sync");
console.log(`registry ${options.registryPath}`);
if (options.dryRun) console.log("mode dry-run");

if (options.targets.has("claudeCode")) syncClaudeCode(servers);
if (options.targets.has("codex")) syncCodex(servers);
if (options.targets.has("opencode")) syncOpenCode(servers);

console.log("");
console.log("Done.");
