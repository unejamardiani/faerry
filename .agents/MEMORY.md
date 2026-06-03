# Faerry Project Memory

## Product Decisions

- Product name: Faerry.
- Name meaning: ferry for transport plus faery/fery hint for a small magic
  association.
- Target users: business users first, with enough detail for developers.
- Product goal: carry agent setup across tools and machines.
- Design direction: Transit OS with Ink + Seafoam palette.
- Faerry must not look like adesso CI/CD.

## Workspace Decisions

- `faerry.json` is the preferred source configuration file.
- Legacy `sources.json` remains readable for compatibility.
- UI edits and migrations should write to `faerry.json`.
- A valid workspace does not require `skills/`; any supported workspace signal
  can be enough.
- Sources should be manageable through the UI, including enable/disable,
  include/exclude, local folders, and Git URLs.

## Release Decisions

- The normal release command is `npm run release`.
- Supported release targets: macOS, Windows, Linux.
- Release artifacts are built per target OS, not cross-compiled by default.
- Each release ships both installable bundles and portable packages:
  - macOS: `.dmg` plus `.app.tar.gz` and `.app.tar.gz.sig` updater artifacts,
    and a portable `.zip` with `Faerry.app`.
  - Windows: NSIS setup `.exe` plus `.exe.sig`, and a portable `.zip` with
    `Faerry.exe`.
  - Linux: `.AppImage` plus `.AppImage.sig`, and a portable `.tar.gz` with
    executable `Faerry`.
- Every portable package gets a `.sha256` sidecar.
- Every release gets a JSON manifest under
  `src-tauri/target/release/bundle/release/`.
- GitHub releases are created by `.github/workflows/release.yml`.
- Pushing a `v*` tag runs the macOS, Windows, and Linux release matrix and
  publishes installers, updater signatures, portable archives, release
  manifests, and the `latest.json` updater manifest.
- The same workflow can also be started manually from the GitHub Actions UI.
- When a coding task is done, run `npm run release` unless explicitly skipped
  or impossible in the current environment.

## Update Decisions

- The in-app updater uses `tauri-plugin-updater` with the GitHub Releases
  endpoint
  `https://github.com/unejamardiani/faerry/releases/latest/download/latest.json`.
- The updater public key is committed in `src-tauri/tauri.conf.json`.
- The updater private key and optional password are GitHub Actions secrets
  `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Windows updater install mode is `passive`, so the NSIS installer runs with
  minimal UI and Faerry restarts after install.
- Faerry runs a quiet update check on startup. The About > Updates panel
  exposes a manual "Check for Updates" action and an "Install Update" action
  that requires user confirmation before installing.

## Verification Baseline

- `npm run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npm run release`

Visual checks should include the Tauri minimum window size `980 x 680` when UI
layout changes are involved.
