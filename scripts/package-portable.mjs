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
const skipBuild = args.has("--no-build");

const platform = process.platform;
const arch = process.arch === "x64" ? "x64" : process.arch;
const portableDir = path.join(tauriDir, "target", "release", "bundle", "portable");
const stageRoot = path.join(os.tmpdir(), `faerry-portable-${process.pid}`);
const stageDir = path.join(stageRoot, `${appName} Portable`);
const outputZip = path.join(
  portableDir,
  `${slug(appName)}_${packageJson.version}_${portablePlatformName(platform)}-${arch}_portable.zip`,
);

if (!["darwin", "win32"].includes(platform)) {
  fail("Portable packaging is currently implemented for macOS and Windows only.");
}

if (!skipBuild) {
  if (platform === "darwin") {
    run("npm", ["run", "tauri", "--", "build", "--bundles", "app"]);
  } else {
    run("npm", ["run", "tauri", "--", "build", "--no-bundle"]);
  }
}

fs.rmSync(stageRoot, { recursive: true, force: true });
fs.mkdirSync(stageDir, { recursive: true });
fs.mkdirSync(portableDir, { recursive: true });

if (platform === "darwin") {
  const appBundle = path.join(tauriDir, "target", "release", "bundle", "macos", `${appName}.app`);
  assertExists(appBundle, "macOS app bundle");
  fs.cpSync(appBundle, path.join(stageDir, `${appName}.app`), { recursive: true });
  writeReadme("Open Faerry.app directly. Drag it to Applications if you want, but no installer is required.");
  run("ditto", ["-c", "-k", "--sequesterRsrc", "--keepParent", stageDir, outputZip], { cwd: stageRoot });
} else {
  const exe = path.join(tauriDir, "target", "release", `${binaryName}.exe`);
  assertExists(exe, "Windows executable");
  fs.copyFileSync(exe, path.join(stageDir, `${appName}.exe`));
  writeReadme("Run Faerry.exe directly. No installer is required.");
  run("powershell.exe", [
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-Command",
    `Compress-Archive -Path ${psQuote(path.join(stageDir, "*"))} -DestinationPath ${psQuote(outputZip)} -Force`,
  ]);
}

fs.rmSync(stageRoot, { recursive: true, force: true });
console.log(`Portable package created: ${outputZip}`);

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
    "- Unsigned or unnotarized builds may show OS trust warnings after download.",
    "",
  ].join("\n");
  fs.writeFileSync(path.join(stageDir, "README.txt"), readme);
}

function run(command, commandArgs, options = {}) {
  const result = spawnSync(command, commandArgs, {
    cwd: options.cwd ?? appRoot,
    stdio: "inherit",
    shell: false,
  });
  if (result.status !== 0) {
    fail(`${command} ${commandArgs.join(" ")} failed with exit ${result.status ?? "unknown"}.`);
  }
}

function assertExists(target, label) {
  if (!fs.existsSync(target)) {
    fail(`Missing ${label}: ${target}. Build first or run without --no-build.`);
  }
}

function slug(value) {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

function portablePlatformName(value) {
  if (value === "darwin") return "macos";
  if (value === "win32") return "windows";
  return value;
}

function psQuote(value) {
  return `'${value.replace(/'/g, "''")}'`;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
