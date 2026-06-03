# DESIGN.md - Faerry Design System

This is the authoritative visual design direction for Faerry.

## Identity

Faerry is a portable agent setup manager. The visual metaphor is transfer,
ferrying, gateways, and calm movement between tools. The product name combines
ferry-like transport with a light faery-like hint, but the interface must stay
credible for business users.

Design direction: Transit OS.

The UI should feel like a quiet control room for setup portability:
- professional
- calm
- direct
- easy for non-technical business users
- still precise enough for developers

## Palette

Use the Ink + Seafoam palette from `src/styles.css`.

Core tokens:
- `--bg: #eef3f0`
- `--bg-rail: #d8e4de`
- `--ink: #10201f`
- `--ink-soft: #2d403d`
- `--muted: #60716c`
- `--surface: #fbfcf8`
- `--surface-muted: #dfe8e2`
- `--surface-raised: #ffffff`
- `--line: #c6d4cd`
- `--accent: #24b8a3`
- `--accent-strong: #0b7f72`
- `--signal: #d99b4e`

Status colors:
- success: `#147462`
- warning: `#8b6123`
- danger: `#9f3d36`
- unknown: `#66736f`

Avoid:
- adesso blue `#006EC7`
- corporate-blue button systems
- purple-blue SaaS gradients
- beige-only or slate-only palettes
- decorative orbs, bokeh blobs, and stock-like abstract backgrounds

## Typography

Use system UI fonts for the interface and monospace only for paths, logs,
diffs, code-like snippets, and checksums.

Do not use adesso Fira Sans Condensed branding.

Headings should be compact and functional. Reserve large display type for real
first-screen product states. Tool panels, tables, modals, and drawers should
use tighter heading scales.

## Layout

The Tauri app window is designed around:
- default size: `1220 x 820`
- minimum size: `980 x 680`
- max content width token: `--container: 1340px`

Use a sticky topbar with clear navigation. The primary views should remain:
- Home
- Sources
- Resources
- Settings

Avoid duplicate source/config displays across views. If users can edit a thing
in one place, other views should link to that place or summarize it.

## Components

Buttons:
- primary for the recommended action
- secondary or ghost for supporting actions
- destructive only for deletion/removal
- disabled states must remain legible

Panels:
- use panels for actual tool surfaces and grouped controls
- do not nest cards inside cards
- keep repeated items compact and scannable

Tables and logs:
- long paths must wrap or scroll without breaking layout
- logs and diffs use monospace
- status rows need clear ok/warn/danger/checking states

Source wizard:
- optimize for business users first
- local folder and Git repository should be obvious modes
- include/exclude controls belong in advanced details
- enabled/disabled state must be editable without touching JSON

Focus:
- preserve visible focus rings
- keyboard navigation should not create layout shift

## Voice

Use plain product language:
- workspace
- source
- portable package
- release
- sync

Avoid old internal wording in visible UI:
- agents repository
- repo validation
- adesso utility

Use technical terms only where they help the user complete the task.
