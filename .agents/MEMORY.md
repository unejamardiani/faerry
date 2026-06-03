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
- Portable packaging outputs:
  - macOS: `.zip` with `Faerry.app`
  - Windows: `.zip` with `Faerry.exe`
  - Linux: `.tar.gz` with executable `Faerry`
- Every portable package gets a `.sha256` sidecar.
- Every release gets a JSON manifest under
  `src-tauri/target/release/bundle/release/`.
- GitHub releases are created by `.github/workflows/release.yml`.
- Pushing a `v*` tag runs the macOS, Windows, and Linux release matrix and
  publishes the portable assets to the GitHub Release page.
- The same workflow can also be started manually from the GitHub Actions UI.
- When a coding task is done, run `npm run release` unless explicitly skipped
  or impossible in the current environment.

## Verification Baseline

- `npm run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npm run release`

Visual checks should include the Tauri minimum window size `980 x 680` when UI
layout changes are involved.
