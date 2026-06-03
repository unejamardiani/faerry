import fs from "node:fs";
import path from "node:path";

const [assetRootArg, releaseTagArg, repoArg] = process.argv.slice(2);

if (!assetRootArg || !releaseTagArg || !repoArg) {
  fail("Usage: node scripts/create-updater-manifest.mjs <asset-root> <release-tag> <owner/repo>");
}

const assetRoot = path.resolve(assetRootArg);
const releaseTag = releaseTagArg.replace(/^refs\/tags\//, "");
const repo = repoArg;
const files = listFiles(assetRoot);
const filesByName = new Map(files.map((file) => [path.basename(file), file]));
const releaseManifests = files
  .filter((file) => file.endsWith("_release.json"))
  .map((file) => JSON.parse(fs.readFileSync(file, "utf8")));

if (releaseManifests.length === 0) {
  fail(`No release manifests found under ${assetRoot}.`);
}

const version = commonVersion(releaseManifests);
const platforms = {};

for (const manifest of releaseManifests) {
  const platformKey = updaterPlatformKey(manifest.platform, manifest.arch);
  const updateArtifact = selectUpdateArtifact(manifest);
  const signatureArtifact = selectSignatureArtifact(manifest, updateArtifact.kind);
  const updateFile = fileForArtifact(updateArtifact);
  const signatureFile = fileForArtifact(signatureArtifact);

  platforms[platformKey] = {
    signature: fs.readFileSync(signatureFile, "utf8").trim(),
    url: `https://github.com/${repo}/releases/download/${releaseTag}/${encodeURIComponent(path.basename(updateFile))}`,
  };
}

const manifest = {
  version,
  notes: `Faerry ${version}`,
  pub_date: new Date().toISOString(),
  platforms,
};

const manifestPath = path.join(assetRoot, "latest.json");
fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Updater manifest created: ${manifestPath}`);

function commonVersion(manifests) {
  const versions = new Set(manifests.map((manifest) => manifest.version));
  if (versions.size !== 1) {
    fail(`Release manifests disagree on version: ${Array.from(versions).join(", ")}`);
  }
  return Array.from(versions)[0];
}

function updaterPlatformKey(platform, arch) {
  const os = platform === "macos" ? "darwin" : platform;
  return `${os}-${updaterArch(arch)}`;
}

function updaterArch(arch) {
  if (arch === "x64") return "x86_64";
  if (arch === "arm64") return "aarch64";
  return arch;
}

function selectUpdateArtifact(manifest) {
  const preferredKinds = {
    macos: "macos-updater",
    windows: "windows-nsis",
    linux: "linux-appimage",
  };
  const kind = preferredKinds[manifest.platform];
  const artifact = manifest.artifacts.find((item) => item.kind === kind);
  if (!artifact) {
    fail(`Missing ${kind} artifact for ${manifest.platform}-${manifest.arch}.`);
  }
  return artifact;
}

function selectSignatureArtifact(manifest, updateKind) {
  const kind = `${updateKind}-signature`;
  const artifact = manifest.artifacts.find((item) => item.kind === kind);
  if (!artifact) {
    fail(`Missing ${kind} artifact for ${manifest.platform}-${manifest.arch}.`);
  }
  return artifact;
}

function fileForArtifact(artifact) {
  const file = filesByName.get(path.basename(artifact.path));
  if (!file) {
    fail(`Release asset not found for ${artifact.path}.`);
  }
  return file;
}

function listFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const child = path.join(dir, entry.name);
    return entry.isDirectory() ? listFiles(child) : [child];
  });
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
