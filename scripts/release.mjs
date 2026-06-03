import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const tauriDir = path.join(appRoot, "src-tauri");
const packageJson = JSON.parse(fs.readFileSync(path.join(appRoot, "package.json"), "utf8"));
const appName = "Faerry";
const binaryName = "faerry";
const args = new Set(process.argv.slice(2));
const platform = process.platform;
const arch = normalizeArch(process.arch);
const executed = [];

if (args.has("--help")) {
  console.log(`Usage: node scripts/release.mjs [--skip-checks] [--skip-tests] [--skip-build]

Run the normal Faerry release process for the current operating system:
  1. TypeScript/Rust check
  2. Rust tests
  3. Tauri release build with installable bundles
  4. Portable package
  5. Release manifest with SHA-256 checksums and updater artifacts

Supported platforms:
  macOS, Windows, Linux

Options:
  --skip-checks  Skip npm run check.
  --skip-tests   Skip cargo test.
  --skip-build   Reuse an existing release build and only package/manifest it.
`);
  process.exit(0);
}

if (!["darwin", "win32", "linux"].includes(platform)) {
  fail(`Release packaging is not implemented for ${platform}.`);
}

syncTauriConfVersion(packageJson.version);

if (!args.has("--skip-checks")) {
  run("npm", ["run", "check"]);
}

if (!args.has("--skip-tests")) {
  run("cargo", ["test", "--manifest-path", "src-tauri/Cargo.toml"]);
}

if (!args.has("--skip-build")) {
  runTauriReleaseBuild();
}

run("npm", ["run", "package:portable", "--", "--no-build"]);

const portableArchive = portableArchivePath();
const portableChecksumPath = `${portableArchive}.sha256`;
assertExists(portableArchive, "portable release artifact");

const manifestDir = path.join(tauriDir, "target", "release", "bundle", "release");
fs.mkdirSync(manifestDir, { recursive: true });
const manifestPath = path.join(
  manifestDir,
  `${slug(appName)}_${packageJson.version}_${portablePlatformName(platform)}-${arch}_release.json`,
);

const artifacts = [
  artifact("portable", portableArchive, true),
  ...nativeBuildArtifacts(),
];

const manifest = {
  productName: appName,
  packageName: packageJson.name,
  version: packageJson.version,
  platform: portablePlatformName(platform),
  arch,
  createdAt: new Date().toISOString(),
  commands: executed,
  artifacts,
};

fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

console.log(`Release manifest created: ${manifestPath}`);
console.log(`Portable checksum: ${portableChecksumPath}`);

function syncTauriConfVersion(version) {
  const tauriConfPath = path.join(tauriDir, "tauri.conf.json");
  const conf = JSON.parse(fs.readFileSync(tauriConfPath, "utf8"));
  if (conf.version !== version) {
    conf.version = version;
    fs.writeFileSync(tauriConfPath, JSON.stringify(conf, null, 2) + "\n");
    console.log(`tauri.conf.json version synced to ${version}`);
  }
}

function runTauriReleaseBuild() {
  run("npm", ["run", "tauri", "--", "build", "--bundles", nativeBundleTargets(platform).join(",")]);
}

function run(command, commandArgs, options = {}) {
  const executable = commandForPlatform(command);
  const label = [command, ...commandArgs].join(" ");
  executed.push(label);
  console.log(`\n> ${label}`);
  const result = spawnSync(executable, commandArgs, {
    cwd: options.cwd ?? appRoot,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  if (result.error) {
    fail(`${label} failed: ${result.error.message}`);
  }
  if (result.status !== 0) {
    fail(`${label} failed with exit ${result.status ?? "unknown"}.`);
  }
}

function commandForPlatform(command) {
  if (process.platform === "win32" && command === "npm") {
    return "npm.cmd";
  }
  return command;
}

function artifact(kind, target, includeChecksum) {
  const stat = fs.statSync(target);
  const item = {
    kind,
    path: path.relative(appRoot, target),
    bytes: stat.isFile() ? stat.size : null,
  };
  if (includeChecksum && stat.isFile()) {
    item.sha256 = sha256File(target);
  }
  return item;
}

function nativeBundleTargets(value) {
  if (value === "darwin") return ["app", "dmg"];
  if (value === "win32") return ["nsis"];
  return ["appimage"];
}

function nativeBuildArtifacts() {
  const bundleRoot = path.join(tauriDir, "target", "release", "bundle");
  if (platform === "darwin") {
    return [
      artifactIfExists("macos-app", path.join(bundleRoot, "macos", `${appName}.app`), false),
      ...artifactsIn(path.join(bundleRoot, "dmg"), (file) => file.endsWith(".dmg"), "macos-dmg"),
      ...artifactsIn(path.join(bundleRoot, "macos"), (file) => file.endsWith(".app.tar.gz"), "macos-updater"),
      ...artifactsIn(path.join(bundleRoot, "macos"), (file) => file.endsWith(".app.tar.gz.sig"), "macos-updater-signature"),
    ].filter(Boolean);
  }
  if (platform === "win32") {
    const nsisDir = path.join(bundleRoot, "nsis");
    return [
      ...artifactsIn(nsisDir, (file) => file.endsWith(".exe"), "windows-nsis"),
      ...artifactsIn(nsisDir, (file) => file.endsWith(".exe.sig"), "windows-nsis-signature"),
    ];
  }
  const appImageDir = path.join(bundleRoot, "appimage");
  return [
    ...artifactsIn(appImageDir, (file) => file.endsWith(".AppImage"), "linux-appimage"),
    ...artifactsIn(appImageDir, (file) => file.endsWith(".AppImage.sig"), "linux-appimage-signature"),
  ];
}

function artifactIfExists(kind, target, includeChecksum) {
  return fs.existsSync(target) ? artifact(kind, target, includeChecksum) : null;
}

function artifactsIn(dir, predicate, kind) {
  if (!fs.existsSync(dir)) return [];
  return listFiles(dir)
    .filter((file) => predicate(path.basename(file)))
    .map((file) => artifact(kind, file, true));
}

function listFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true })
    .flatMap((entry) => {
      const child = path.join(dir, entry.name);
      return entry.isDirectory() ? listFiles(child) : [child];
    });
}

function portableArchivePath() {
  return path.join(
    tauriDir,
    "target",
    "release",
    "bundle",
    "portable",
    `${slug(appName)}_${packageJson.version}_${portablePlatformName(platform)}-${arch}_portable${archiveExtension(platform)}`,
  );
}

function assertExists(target, label) {
  if (!fs.existsSync(target)) {
    fail(`Missing ${label}: ${target}.`);
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

function fail(message) {
  console.error(message);
  process.exit(1);
}
