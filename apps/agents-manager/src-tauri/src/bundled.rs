use crate::repo;
use std::{
    fs,
    path::{Path, PathBuf},
};

struct BundledScript {
    name: &'static str,
    content: &'static str,
    executable: bool,
}

const SCRIPTS: &[BundledScript] = &[
    BundledScript {
        name: "link-agents.sh",
        content: include_str!("../../../../scripts/link-agents.sh"),
        executable: true,
    },
    BundledScript {
        name: "sync-all-agents.sh",
        content: include_str!("../../../../scripts/sync-all-agents.sh"),
        executable: true,
    },
    BundledScript {
        name: "sync-mcps.mjs",
        content: include_str!("../../../../scripts/sync-mcps.mjs"),
        executable: true,
    },
    BundledScript {
        name: "package-claude-skills.sh",
        content: include_str!("../../../../scripts/package-claude-skills.sh"),
        executable: true,
    },
    BundledScript {
        name: "package-claude-extension.sh",
        content: include_str!("../../../../scripts/package-claude-extension.sh"),
        executable: true,
    },
    BundledScript {
        name: "sync-claude-cowork-skills.mjs",
        content: include_str!("../../../../scripts/sync-claude-cowork-skills.mjs"),
        executable: true,
    },
    BundledScript {
        name: "sync-claude-desktop-extension.mjs",
        content: include_str!("../../../../scripts/sync-claude-desktop-extension.mjs"),
        executable: true,
    },
    BundledScript {
        name: "link-agents.ps1",
        content: include_str!("../../../../scripts/link-agents.ps1"),
        executable: false,
    },
    BundledScript {
        name: "sync-all-agents.ps1",
        content: include_str!("../../../../scripts/sync-all-agents.ps1"),
        executable: false,
    },
    BundledScript {
        name: "package-claude-skills.ps1",
        content: include_str!("../../../../scripts/package-claude-skills.ps1"),
        executable: false,
    },
    BundledScript {
        name: "package-claude-extension.ps1",
        content: include_str!("../../../../scripts/package-claude-extension.ps1"),
        executable: false,
    },
];

pub fn materialize() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir()
        .join("portable-agents-manager")
        .join(format!("scripts-v{}", env!("CARGO_PKG_VERSION")));
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;

    for script in SCRIPTS {
        let path = dir.join(script.name);
        fs::write(&path, patched_content(script.name, script.content)).map_err(|error| error.to_string())?;
        set_permissions(&path, script.executable)?;
    }

    Ok(dir)
}

pub fn script_path(name: &str) -> Result<PathBuf, String> {
    Ok(materialize()?.join(name))
}

pub fn display_name(command: &str) -> Option<&str> {
    command.strip_prefix("bundled:")
}

pub fn get_script_content(name: &str) -> String {
    SCRIPTS.iter()
        .find(|s| s.name == name)
        .map(|s| s.content.to_string())
        .unwrap_or_default()
}

fn patched_content(name: &str, content: &str) -> String {
    match name {
        "sync-all-agents.sh" => content
            .replace(
                "SCRIPT_DIR=\"$REPO_ROOT/scripts\"",
                "export PORTABLE_AGENTS_REPO_ROOT=\"$REPO_ROOT\"\nSCRIPT_DIR=$(cd -- \"$(dirname -- \"${BASH_SOURCE[0]}\")\" && pwd)",
            )
            .replace(
                "node \"$SCRIPT_DIR/sync-mcps.mjs\" --dry-run",
                "node \"$SCRIPT_DIR/sync-mcps.mjs\" --registry \"$REPO_ROOT/mcp/servers.json\" --dry-run",
            )
            .replace(
                "node \"$SCRIPT_DIR/sync-mcps.mjs\"",
                "node \"$SCRIPT_DIR/sync-mcps.mjs\" --registry \"$REPO_ROOT/mcp/servers.json\"",
            ),
        "package-claude-skills.sh" | "package-claude-extension.sh" => content.replace(
            "REPO_ROOT=$(cd -- \"$SCRIPT_DIR/..\" && pwd)",
            "REPO_ROOT=\"${PORTABLE_AGENTS_REPO_ROOT:-$(cd -- \"$SCRIPT_DIR/..\" && pwd)}\"",
        ),
        "sync-claude-cowork-skills.mjs" => content.replace(
            "const repoRoot = path.resolve(scriptDir, \"..\");",
            "const repoRoot = process.env.PORTABLE_AGENTS_REPO_ROOT || path.resolve(scriptDir, \"..\");",
        ),
        "sync-claude-desktop-extension.mjs" => content.replace(
            "const repoRoot = path.resolve(__dirname, \"..\");",
            "const repoRoot = process.env.PORTABLE_AGENTS_REPO_ROOT || path.resolve(__dirname, \"..\");",
        ),
        _ => content.to_string(),
    }
}

#[cfg(unix)]
fn set_permissions(path: &Path, executable: bool) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    if executable {
        let mut permissions = fs::metadata(path).map_err(|error| error.to_string())?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_permissions(_path: &Path, _executable: bool) -> Result<(), String> {
    Ok(())
}

pub fn bundled_command_label(command: &str) -> String {
    display_name(command)
        .map(|name| format!("<bundled-scripts>/{name}"))
        .unwrap_or_else(|| command.to_string())
}

pub fn bundled_script_exists(name: &str) -> bool {
    SCRIPTS.iter().any(|script| script.name == name)
}

pub fn display_path(path: impl AsRef<Path>) -> String {
    repo::display_path(path)
}
