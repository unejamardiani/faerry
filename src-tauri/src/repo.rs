use crate::models::{AgentsRepo, RepoPaths};
use std::{
    env,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct RepoError(String);

impl fmt::Display for RepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for RepoError {}

pub fn detect_repo_with_override(repo_override: Option<String>) -> Result<AgentsRepo, RepoError> {
    let home = home_dir().ok_or_else(|| RepoError("Unable to determine home directory.".into()))?;

    if let Some(repo_override) = repo_override {
        if !repo_override.trim().is_empty() {
            let resolved = expand_home(repo_override.trim(), &home);
            if is_repo(&resolved) {
                return Ok(make_repo(resolved, home));
            }
            return Err(RepoError(format!(
                "Selected path is not a portable agents repo: {}",
                display_path(resolved)
            )));
        }
    }

    if let Ok(explicit) = env::var("AGENTS_REPO").or_else(|_| env::var("AGENTS_HOME")) {
        if !explicit.trim().is_empty() {
            let resolved = expand_home(&explicit, &home);
            if is_repo(&resolved) {
                return Ok(make_repo(resolved, home));
            }
        }
    }

    if let Ok(current) = env::current_dir() {
        if let Some(repo) = walk_for_repo(&current, &home) {
            return Ok(repo);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(repo) = walk_for_repo(&manifest_dir, &home) {
        return Ok(repo);
    }

    let home_repo = home.join(".agents");
    if is_repo(&home_repo) {
        return Ok(make_repo(home_repo, home));
    }

    Err(RepoError("No portable agents repo found. Set AGENTS_REPO or run from inside a repo containing AGENTS.md, skills, commands, and mcp/servers.json.".into()))
}

pub fn read_text(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok()
}

pub fn path_exists(path: impl AsRef<Path>) -> bool {
    fs::symlink_metadata(path).is_ok()
}

pub fn list_dirs(path: impl AsRef<Path>) -> Vec<String> {
    let mut entries: Vec<String> = fs::read_dir(path)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if file_type.is_dir() {
                Some(entry.file_name().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    entries.sort();
    entries
}

pub fn list_markdown_files(path: impl AsRef<Path>) -> Vec<String> {
    let mut entries: Vec<String> = fs::read_dir(path)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if file_type.is_file() && name.ends_with(".md") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    entries.sort();
    entries
}

pub fn parse_frontmatter(markdown: &str) -> std::collections::BTreeMap<String, String> {
    let mut result = std::collections::BTreeMap::new();
    if !markdown.starts_with("---\n") {
        return result;
    }
    let Some(end) = markdown[4..].find("\n---") else {
        return result;
    };
    let block = &markdown[4..4 + end];
    let lines: Vec<&str> = block.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let Some((key, raw_value)) = line.split_once(':') else {
            index += 1;
            continue;
        };
        let mut value = raw_value.trim().to_string();
        if value == ">" || value == "|" {
            let mut parts = Vec::new();
            index += 1;
            while index < lines.len() {
                let next = lines[index];
                if next
                    .chars()
                    .next()
                    .is_some_and(|char| !char.is_whitespace())
                    && next.contains(':')
                {
                    index -= 1;
                    break;
                }
                parts.push(next.trim());
                index += 1;
            }
            value = parts
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
        }
        result.insert(
            key.trim().to_string(),
            value.trim_matches(&['"', '\''][..]).to_string(),
        );
        index += 1;
    }
    result
}

pub fn resolve_user_path(value: &str) -> Result<PathBuf, String> {
    let home = home_dir().ok_or_else(|| "Unable to determine home directory.".to_string())?;
    Ok(expand_home(value.trim(), &home))
}

pub fn is_agents_repo(candidate: impl AsRef<Path>) -> bool {
    is_repo(candidate.as_ref())
}

fn home_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        env::var_os("USERPROFILE").map(PathBuf::from)
    } else {
        env::var_os("HOME").map(PathBuf::from)
    }
}

fn walk_for_repo(start: &Path, home: &Path) -> Option<AgentsRepo> {
    let mut current = start.to_path_buf();
    loop {
        if is_repo(&current) {
            return Some(make_repo(current, home.to_path_buf()));
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn expand_home(value: &str, home: &Path) -> PathBuf {
    if value == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return home.join(rest);
    }
    PathBuf::from(value)
}

fn is_repo(candidate: &Path) -> bool {
    ["AGENTS.md", "skills", "mcp/servers.json"]
        .iter()
        .all(|entry| candidate.join(entry).exists())
}

fn make_repo(root: PathBuf, home: PathBuf) -> AgentsRepo {
    let root = normalize(root);
    let home = normalize(home);
    let agents_home = home.join(".agents");
    let codex_home = env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"));
    AgentsRepo {
        ok: true,
        root: display_path(&root),
        home: display_path(&home),
        agents_home: display_path(&agents_home),
        codex_home: display_path(&codex_home),
        paths: RepoPaths {
            agents: display_path(root.join("AGENTS.md")),
            skills: display_path(root.join("skills")),
            commands: display_path(root.join("commands")),
            designs: display_path(root.join("designs")),
            registry: display_path(root.join("mcp/servers.json")),
            scripts: display_path(root.join("scripts")),
        },
    }
}

pub fn display_path(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

fn normalize(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}
