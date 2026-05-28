use crate::bundled;
use crate::models::{BundledScriptInfo, ScriptVersionInfo, ScriptVersionStatus};
use crate::repo;
use std::fs;
use std::path::Path;

const SCRIPT_NAMES: &[&str] = &[
    "link-agents.sh",
    "sync-all-agents.sh",
    "sync-mcps.mjs",
    "sync-source-skills.mjs",
    "package-claude-skills.sh",
    "package-claude-extension.sh",
    "sync-claude-cowork-skills.mjs",
    "sync-claude-desktop-extension.mjs",
    "link-agents.ps1",
    "sync-all-agents.ps1",
    "package-claude-skills.ps1",
    "package-claude-extension.ps1",
];

pub fn get_bundled_scripts() -> Vec<BundledScriptInfo> {
    SCRIPT_NAMES
        .iter()
        .map(|name| {
            let content = bundled::get_script_content(name);
            let checksum = hash_content(&content);
            BundledScriptInfo {
                name: name.to_string(),
                checksum,
                size: content.len(),
            }
        })
        .collect()
}

pub fn get_script_versions(repo_root: &str) -> Vec<ScriptVersionInfo> {
    let repo_path = Path::new(repo_root);
    let script_dir = repo_path.join("scripts");

    SCRIPT_NAMES
        .iter()
        .map(|name| {
            let bundled_content = bundled::get_script_content(name);
            let bundled_checksum = hash_content(&bundled_content);
            let repo_script = script_dir.join(name);
            let repo_exists = repo_script.exists();

            let (repo_checksum, repo_size) = if repo_exists {
                if let Ok(content) = fs::read_to_string(&repo_script) {
                    (Some(hash_content(&content)), Some(content.len() as u64))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            let status = if let Some(ref rc) = repo_checksum {
                if *rc == bundled_checksum {
                    ScriptVersionStatus::BundledMatchesRepo
                } else {
                    ScriptVersionStatus::BundledDiffersFromRepo
                }
            } else if repo_exists {
                ScriptVersionStatus::RepoMissing
            } else {
                ScriptVersionStatus::NoRepoScript
            };

            ScriptVersionInfo {
                name: name.to_string(),
                bundled_checksum: bundled_checksum.clone(),
                bundled_size: bundled_content.len(),
                repo_path: repo_exists.then(|| repo::display_path(&repo_script)),
                repo_checksum,
                repo_size,
                status,
            }
        })
        .collect()
}

fn hash_content(content: &str) -> String {
    let mut state = std::collections::hash_map::DefaultHasher::new();
    use std::hash::{Hash, Hasher};
    content.hash(&mut state);
    format!("{:016x}", state.finish())
}
