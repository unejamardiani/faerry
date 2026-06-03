import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const tauriDir = path.join(appRoot, "src-tauri");
const packageJson = JSON.parse(fs.readFileSync(path.join(appRoot, "package.json"), "utf8"));
const appName = "Faerry";
const binaryName = "faerry";
const args = new Set(process.argv.slice(2));

if (args.has("--help")) {
  console.log(`Usage: node scripts/package-portable.mjs [--no-build]

Create a portable Faerry package for the current operating system.

Supported platforms:
  macOS    .zip containing Faerry.app
  Windows  .zip containing Faerry.exe
  Linux    .tar.gz containing the Faerry executable

Options:
  --no-build  Package an existing release build instead of running Tauri build.
`);
  process.exit(0);
}

const skipBuild = args.has("--no-build");
const platform = process.platform;
const arch = normalizeArch(process.arch);
const portableDir = path.join(tauriDir, "target", "release", "bundle", "portable");
const stageRoot = path.join(os.tmpdir(), `faerry-portable-${process.pid}`);
const stageName = `${appName} Portable`;
const stageDir = path.join(stageRoot, stageName);
const outputArchive = path.join(
  portableDir,
  `${slug(appName)}_${packageJson.version}_${portablePlatformName(platform)}-${arch}_portable${archiveExtension(platform)}`,
);
const checksumPath = `${outputArchive}.sha256`;

if (!["darwin", "win32", "linux"].includes(platform)) {
  fail(`Portable packaging is not implemented for ${platform}.`);
}

if (!skipBuild) {
  runTauriBuild();
}

fs.rmSync(stageRoot, { recursive: true, force: true });
fs.rmSync(outputArchive, { force: true });
fs.rmSync(checksumPath, { force: true });
fs.mkdirSync(portableDir, { recursive: true });
fs.mkdirSync(stageDir, { recursive: true });

if (platform === "darwin") {
  const appBundle = path.join(tauriDir, "target", "release", "bundle", "macos", `${appName}.app`);
  assertExists(appBundle, "macOS app bundle");
  fs.cpSync(appBundle, path.join(stageDir, `${appName}.app`), { recursive: true });
  writeReadme("Open Faerry.app directly. Drag it to Applications if you want, but no installer is required.");
  run("ditto", ["-c", "-k", "--sequesterRsrc", "--keepParent", stageName, outputArchive], { cwd: stageRoot });
} else if (platform === "win32") {
  const exe = path.join(tauriDir, "target", "release", `${binaryName}.exe`);
  assertExists(exe, "Windows executable");
  fs.copyFileSync(exe, path.join(stageDir, `${appName}.exe`));
  writeReadme("Run Faerry.exe directly. No installer is required.");
  run("powershell.exe", [
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-Command",
    `Compress-Archive -Path ${psQuote(stageDir)} -DestinationPath ${psQuote(outputArchive)} -Force`,
  ]);
} else {
  const binary = path.join(tauriDir, "target", "release", binaryName);
  assertExists(binary, "Linux executable");
  const portableBinary = path.join(stageDir, appName);
  fs.copyFileSync(binary, portableBinary);
  fs.chmodSync(portableBinary, 0o755);
  writeReadme("Run ./Faerry directly from a terminal or file manager. No installer is required.");
  run("tar", ["-czf", outputArchive, "-C", stageRoot, stageName]);
}

const checksum = sha256File(outputArchive);
fs.writeFileSync(checksumPath, `${checksum}  ${path.basename(outputArchive)}\n`);
fs.rmSync(stageRoot, { recursive: true, force: true });

console.log(`Portable package created: ${outputArchive}`);
console.log(`SHA-256 checksum: ${checksumPath}`);

function runTauriBuild() {
  if (platform === "darwin") {
    run("npm", ["run", "tauri", "--", "build", "--bundles", "app"]);
    return;
  }
  run("npm", ["run", "tauri", "--", "build", "--no-bundle"]);
}

function writeReadme(openingLine) {
  const readme = [
    `${appName} Portable`,
    "",
    openingLine,
    "",
    "Notes:",
    "- This package contains the manager app only.",
    "- Sync features still depend on the target machine having the relevant CLIs available, such as node, git, codex, claude, or opencode.",
    "- Windows requires Microsoft Edge WebView2 Runtime. Windows 11 includes it; older Windows machines may need it installed separately.",
    "- Linux requires the usual WebKitGTK/Tauri runtime libraries for desktop apps.",
    "- Unsigned or unnotarized builds may show OS trust warnings after download.",
    "",
  ].join("\n");
  fs.writeFileSync(path.join(stageDir, "README.txt"), readme);
}

function run(command, commandArgs, options = {}) {
  const executable = commandForPlatform(command);
  const result = spawnSync(executable, commandArgs, {
    cwd: options.cwd ?? appRoot,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  if (result.error) {
    fail(`${command} ${commandArgs.join(" ")} failed: ${result.error.message}`);
  }
  if (result.status !== 0) {
    fail(`${command} ${commandArgs.join(" ")} failed with exit ${result.status ?? "unknown"}.`);
  }
}

function commandForPlatform(command) {
  if (process.platform === "win32" && command === "npm") {
    return "npm.cmd";
  }
  return command;
}

function assertExists(target, label) {
  if (!fs.existsSync(target)) {
    fail(`Missing ${label}: ${target}. Build first or run without --no-build.`);
  }
}

function sha256File(target) {
  const hash = createHash("sha256");
  hash.update(fs.readFileSync(target));
  return hash.digest("hex");
}

function slug(value) {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

function archiveExtension(value) {
  return value === "linux" ? ".tar.gz" : ".zip";
}

function portablePlatformName(value) {
  if (value === "darwin") return "macos";
  if (value === "win32") return "windows";
  return value;
}

function normalizeArch(value) {
  if (value === "x64") return "x64";
  if (value === "arm64") return "arm64";
  return value;
}

function psQuote(value) {
  return `'${value.replace(/'/g, "''")}'`;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
