use crate::{auth, models::*, repo};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

static AUTH_STATUS_CACHE: OnceLock<Mutex<BTreeMap<String, (Instant, String)>>> = OnceLock::new();

pub fn build_state(repo_override: Option<String>) -> AppState {
    match repo::detect_repo_with_override(repo_override) {
        Ok(repo) => {
            let mut source_load = read_source_config(&repo);
            let registry = read_registry(&repo);
            let tools = read_tools(&repo);
            let skill_installs = installs_for(&tools, "skills");
            let command_installs = installs_for(&tools, "commands");
            let skills = read_skills(
                &repo,
                skill_installs,
                &source_load.sources,
                &mut source_load.status.warnings,
            );
            let commands = read_commands(
                &repo,
                command_installs,
                &source_load.sources,
                &mut source_load.status.warnings,
            );
            let designs = read_designs(
                &repo,
                &source_load.sources,
                &mut source_load.status.warnings,
            );
            let mcp_statuses = read_mcp_statuses(&repo, &registry.servers);
            AppState {
                repo: Some(repo),
                repo_error: None,
                generated_at: generated_at(),
                source_config: source_load.status,
                registry,
                tools,
                skills,
                commands,
                designs,
                mcp_statuses,
            }
        }
        Err(error) => AppState {
            repo: None,
            repo_error: Some(error.to_string()),
            generated_at: generated_at(),
            source_config: SourceConfigStatus {
                path: String::new(),
                standard_path: String::new(),
                legacy: false,
                exists: false,
                valid: false,
                error: Some(error.to_string()),
                sources: Vec::new(),
                warnings: Vec::new(),
            },
            registry: McpRegistry {
                valid: false,
                path: String::new(),
                error: Some(error.to_string()),
                servers: Vec::new(),
            },
            tools: Vec::new(),
            skills: Vec::new(),
            commands: Vec::new(),
            designs: Vec::new(),
            mcp_statuses: BTreeMap::new(),
        },
    }
}

fn generated_at() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    seconds.to_string()
}

#[derive(Debug, Clone)]
struct SourceLoad {
    status: SourceConfigStatus,
    sources: Vec<LoadedResourceSource>,
}

#[derive(Debug, Clone)]
struct LoadedResourceSource {
    name: String,
    resolved_path: PathBuf,
    source_kind: String,
    skills_dir: Option<PathBuf>,
    skill_dirs: Vec<PathBuf>,
    commands_dir: Option<PathBuf>,
    command_files: Vec<PathBuf>,
    designs_dir: Option<PathBuf>,
    design_files: Vec<PathBuf>,
    include_skills: Vec<String>,
    exclude_skills: Vec<String>,
    include_commands: Vec<String>,
    exclude_commands: Vec<String>,
    include_designs: Vec<String>,
    exclude_designs: Vec<String>,
}

#[derive(Debug, Clone)]
struct ResolvedSourceRoot {
    path: PathBuf,
    source_kind: String,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct GitSourceSpec {
    clone_url: String,
    git_ref: Option<String>,
    subpath: Option<PathBuf>,
}

fn read_source_config(repo: &AgentsRepo) -> SourceLoad {
    let root = Path::new(&repo.root);
    let path = repo::source_config_path(root);
    let standard_path = repo::standard_source_config_path(root);
    let display_path = repo::display_path(&path);
    let mut status = SourceConfigStatus {
        path: display_path,
        standard_path: repo::display_path(standard_path),
        legacy: repo::is_legacy_source_config(&path),
        exists: path.exists(),
        valid: true,
        error: None,
        sources: Vec::new(),
        warnings: Vec::new(),
    };

    if !status.exists {
        return SourceLoad {
            status,
            sources: Vec::new(),
        };
    }

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            status.valid = false;
            status.error = Some(error.to_string());
            return SourceLoad {
                status,
                sources: Vec::new(),
            };
        }
    };
    let parsed: ResourceSourcesFile = match serde_json::from_str(&text) {
        Ok(parsed) => parsed,
        Err(error) => {
            status.valid = false;
            status.error = Some(error.to_string());
            return SourceLoad {
                status,
                sources: Vec::new(),
            };
        }
    };

    let mut loaded = Vec::new();
    for (index, source) in parsed.sources.iter().enumerate() {
        let enabled = source.enabled.unwrap_or(true);
        let name = source
            .name
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("source-{}", index + 1));
        let source_display = source
            .url
            .as_deref()
            .or(source.path.as_deref())
            .unwrap_or_default()
            .to_string();
        let resources = enabled_resources(source);
        let skills_enabled = source.skills.unwrap_or(true);
        let commands_enabled = source.commands.unwrap_or(true);
        let designs_enabled = source.designs.unwrap_or(false);
        let mut source_status = ResourceSourceStatus {
            index,
            name: name.clone(),
            path: source_display,
            url: source.url.clone().unwrap_or_default(),
            git_ref: source
                .git_ref
                .clone()
                .or_else(|| source.branch.clone())
                .unwrap_or_default(),
            refresh: source.refresh.unwrap_or(false),
            resolved_path: String::new(),
            enabled,
            skills: skills_enabled,
            commands: commands_enabled,
            designs: designs_enabled,
            skills_path: source.skills_path.clone().unwrap_or_default(),
            commands_path: source.commands_path.clone().unwrap_or_default(),
            designs_path: source.designs_path.clone().unwrap_or_default(),
            skill_paths: source.skill_paths.clone(),
            command_paths: source.command_paths.clone(),
            design_paths: source.design_paths.clone(),
            include_skills: source.include_skills.clone(),
            exclude_skills: source.exclude_skills.clone(),
            include_commands: source.include_commands.clone(),
            exclude_commands: source.exclude_commands.clone(),
            include_designs: source.include_designs.clone(),
            exclude_designs: source.exclude_designs.clone(),
            resources: resources.clone(),
            status: "disabled".into(),
            message: "Source is disabled.".into(),
        };

        if !enabled {
            status.sources.push(source_status);
            continue;
        }
        let resolved = match resolve_source_root(repo, source, &name) {
            Ok(resolved) => resolved,
            Err(error) => {
                source_status.status = "error".into();
                source_status.message = error.clone();
                status
                    .warnings
                    .push(format!("Source `{name}` was skipped: {error}"));
                status.sources.push(source_status);
                continue;
            }
        };
        for warning in &resolved.warnings {
            status.warnings.push(format!("Source `{name}`: {warning}"));
        }
        let resolved_path = resolved.path;
        source_status.resolved_path = repo::display_path(&resolved_path);
        if resources.is_empty() {
            source_status.status = "warning".into();
            source_status.message = "No resource categories are enabled.".into();
            status.warnings.push(format!(
                "Source `{name}` has no enabled resource categories."
            ));
            status.sources.push(source_status);
            continue;
        }

        let (skills_dir, skill_dirs) = if resources.iter().any(|resource| resource == "skills") {
            resolve_skill_paths(&resolved_path, source)
        } else {
            (None, Vec::new())
        };
        let (commands_dir, command_files) =
            if resources.iter().any(|resource| resource == "commands") {
                resolve_command_paths(&resolved_path, source)
            } else {
                (None, Vec::new())
            };
        let (designs_dir, design_files) = if resources.iter().any(|resource| resource == "designs")
        {
            resolve_design_paths(&resolved_path, source)
        } else {
            (None, Vec::new())
        };

        if skills_dir.is_none()
            && skill_dirs.is_empty()
            && commands_dir.is_none()
            && command_files.is_empty()
            && designs_dir.is_none()
            && design_files.is_empty()
        {
            source_status.status = "warning".into();
            source_status.message =
                "Source path exists, but no enabled resource directories were found.".into();
            status.warnings.push(format!(
                "Source `{name}` has no enabled resource directories under {}.",
                repo::display_path(&resolved_path)
            ));
            status.sources.push(source_status);
            continue;
        }

        let mut loaded_resources = Vec::new();
        if skills_dir.is_some() || !skill_dirs.is_empty() {
            loaded_resources.push("skills");
        }
        if commands_dir.is_some() || !command_files.is_empty() {
            loaded_resources.push("commands");
        }
        if designs_dir.is_some() || !design_files.is_empty() {
            loaded_resources.push("designs");
        }
        source_status.status = if resolved.warnings.is_empty() {
            "loaded".into()
        } else {
            "warning".into()
        };
        source_status.message = format!("Loaded {}.", loaded_resources.join(", "));
        source_status.resources = loaded_resources
            .into_iter()
            .map(ToString::to_string)
            .collect();
        status.sources.push(source_status);
        loaded.push(LoadedResourceSource {
            name,
            resolved_path,
            source_kind: resolved.source_kind,
            skills_dir,
            skill_dirs,
            commands_dir,
            command_files,
            designs_dir,
            design_files,
            include_skills: source.include_skills.clone(),
            exclude_skills: source.exclude_skills.clone(),
            include_commands: source.include_commands.clone(),
            exclude_commands: source.exclude_commands.clone(),
            include_designs: source.include_designs.clone(),
            exclude_designs: source.exclude_designs.clone(),
        });
    }

    SourceLoad {
        status,
        sources: loaded,
    }
}

fn resolve_source_root(
    repo: &AgentsRepo,
    source: &ResourceSourceConfig,
    name: &str,
) -> Result<ResolvedSourceRoot, String> {
    if let Some(url) = source
        .url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return resolve_git_source(repo, source, name, url);
    }

    let Some(path) = source
        .path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        return Err("Either path or url is required.".into());
    };
    let resolved_path = resolve_local_source_path(repo, path);
    if !resolved_path.exists() {
        return Err(format!(
            "Resolved source path does not exist: {}",
            repo::display_path(&resolved_path)
        ));
    }
    Ok(ResolvedSourceRoot {
        path: resolved_path,
        source_kind: "external".into(),
        warnings: Vec::new(),
    })
}

fn enabled_resources(source: &ResourceSourceConfig) -> Vec<String> {
    let skills = source.skills.unwrap_or(true);
    let commands = source.commands.unwrap_or(true);
    let designs = source.designs.unwrap_or(false);
    let mut resources = Vec::new();
    if skills {
        resources.push("skills".into());
    }
    if commands {
        resources.push("commands".into());
    }
    if designs {
        resources.push("designs".into());
    }
    resources
}

fn resolve_local_source_path(repo: &AgentsRepo, path: &str) -> PathBuf {
    let raw = path.trim();
    let expanded = repo::resolve_user_path(raw).unwrap_or_else(|_| PathBuf::from(raw));
    let resolved = if expanded.is_absolute() {
        expanded
    } else {
        Path::new(&repo.root).join(expanded)
    };
    resolved.canonicalize().unwrap_or(resolved)
}

fn resolve_git_source(
    repo: &AgentsRepo,
    source: &ResourceSourceConfig,
    name: &str,
    url: &str,
) -> Result<ResolvedSourceRoot, String> {
    let explicit_ref = source
        .git_ref
        .clone()
        .or_else(|| source.branch.clone())
        .filter(|value| !value.trim().is_empty());
    let spec = git_source_spec(url, explicit_ref);
    let cache_path = git_cache_path(repo, name, &spec);
    let warnings = ensure_git_cache(&spec, &cache_path, source.refresh.unwrap_or(false))?;
    let source_path = spec
        .subpath
        .as_ref()
        .map(|subpath| cache_path.join(subpath))
        .unwrap_or(cache_path);
    if !source_path.exists() {
        return Err(format!(
            "Git source was fetched, but the configured path does not exist: {}",
            repo::display_path(&source_path)
        ));
    }
    Ok(ResolvedSourceRoot {
        path: source_path.canonicalize().unwrap_or(source_path),
        source_kind: "git".into(),
        warnings,
    })
}

fn git_source_spec(url: &str, explicit_ref: Option<String>) -> GitSourceSpec {
    if let Some(spec) = parse_github_web_url(url, explicit_ref.clone()) {
        return spec;
    }
    GitSourceSpec {
        clone_url: url.trim().to_string(),
        git_ref: explicit_ref,
        subpath: None,
    }
}

fn parse_github_web_url(url: &str, explicit_ref: Option<String>) -> Option<GitSourceSpec> {
    let clean = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .trim_end_matches('/');
    let rest = clean
        .strip_prefix("https://github.com/")
        .or_else(|| clean.strip_prefix("http://github.com/"))
        .or_else(|| clean.strip_prefix("https://www.github.com/"))
        .or_else(|| clean.strip_prefix("http://www.github.com/"))?;
    let parts: Vec<&str> = rest.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let owner = parts[0];
    let repo_name = parts[1].trim_end_matches(".git");
    let clone_url = format!("https://github.com/{owner}/{repo_name}.git");
    if parts.len() >= 4 && (parts[2] == "tree" || parts[2] == "blob") {
        let git_ref = explicit_ref.or_else(|| Some(parts[3].to_string()));
        let mut subpath_parts: Vec<&str> = parts[4..].to_vec();
        if parts[2] == "blob" && subpath_parts.last() == Some(&"SKILL.md") {
            subpath_parts.pop();
        }
        let subpath = (!subpath_parts.is_empty()).then(|| {
            let mut path = PathBuf::new();
            for part in subpath_parts {
                path.push(part);
            }
            path
        });
        return Some(GitSourceSpec {
            clone_url,
            git_ref,
            subpath,
        });
    }
    Some(GitSourceSpec {
        clone_url,
        git_ref: explicit_ref,
        subpath: None,
    })
}

fn git_cache_path(repo: &AgentsRepo, name: &str, spec: &GitSourceSpec) -> PathBuf {
    let key = format!(
        "{}|{}|{}",
        spec.clone_url,
        spec.git_ref.as_deref().unwrap_or_default(),
        spec.subpath
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    );
    Path::new(&repo.root)
        .join(".agents-manager")
        .join("source-cache")
        .join(format!("{}-{}", slugify(name), stable_hash(&key)))
}

fn ensure_git_cache(
    spec: &GitSourceSpec,
    cache_path: &Path,
    refresh: bool,
) -> Result<Vec<String>, String> {
    let mut warnings = Vec::new();
    let git_dir = cache_path.join(".git");
    if git_dir.is_dir() {
        if refresh {
            let mut fetch = Command::new("git");
            fetch
                .arg("-C")
                .arg(cache_path)
                .arg("fetch")
                .arg("--depth")
                .arg("1");
            if let Some(git_ref) = &spec.git_ref {
                fetch.arg("origin").arg(git_ref);
            }
            let output = fetch.output().map_err(|error| error.to_string())?;
            if !output.status.success() {
                warnings.push(format!(
                    "Git refresh failed; using existing cache. {}",
                    command_error(&output)
                ));
            }
        }
        if let Some(git_ref) = &spec.git_ref {
            let output = Command::new("git")
                .arg("-C")
                .arg(cache_path)
                .arg("checkout")
                .arg(git_ref)
                .output()
                .map_err(|error| error.to_string())?;
            if !output.status.success() {
                warnings.push(format!(
                    "Git checkout `{git_ref}` failed; using existing checkout. {}",
                    command_error(&output)
                ));
            }
        }
        return Ok(warnings);
    }
    if cache_path.exists() {
        return Err(format!(
            "Git cache path exists but is not a Git checkout: {}",
            repo::display_path(cache_path)
        ));
    }

    let parent = cache_path
        .parent()
        .ok_or_else(|| "Git cache path has no parent directory.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut clone = Command::new("git");
    clone.arg("clone").arg("--depth").arg("1");
    if let Some(git_ref) = &spec.git_ref {
        clone.arg("--branch").arg(git_ref);
    }
    let output = clone
        .arg(&spec.clone_url)
        .arg(cache_path)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(warnings)
    } else {
        Err(format!("Git clone failed. {}", command_error(&output)))
    }
}

fn command_error(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    if detail.is_empty() {
        format!("exit code: {}", output.status)
    } else {
        detail.to_string()
    }
}

fn resolve_skill_paths(
    root: &Path,
    source: &ResourceSourceConfig,
) -> (Option<PathBuf>, Vec<PathBuf>) {
    if !source.skill_paths.is_empty() {
        return (
            None,
            source
                .skill_paths
                .iter()
                .map(|path| join_source_child(root, path))
                .filter(|path| path.join("SKILL.md").is_file())
                .collect(),
        );
    }
    if let Some(skills_path) = source.skills_path.as_deref() {
        let path = join_source_child(root, skills_path);
        if path.join("SKILL.md").is_file() {
            return (None, vec![path]);
        }
        return (path.is_dir().then_some(path), Vec::new());
    }
    let conventional = root.join("skills");
    if conventional.is_dir() {
        return (Some(conventional), Vec::new());
    }
    if root.join("SKILL.md").is_file() {
        return (None, vec![root.to_path_buf()]);
    }
    if has_skill_children(root) {
        return (Some(root.to_path_buf()), Vec::new());
    }
    (None, Vec::new())
}

fn resolve_command_paths(
    root: &Path,
    source: &ResourceSourceConfig,
) -> (Option<PathBuf>, Vec<PathBuf>) {
    if !source.command_paths.is_empty() {
        return (
            None,
            source
                .command_paths
                .iter()
                .map(|path| join_source_child(root, path))
                .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "md"))
                .collect(),
        );
    }
    if let Some(commands_path) = source.commands_path.as_deref() {
        let path = join_source_child(root, commands_path);
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            return (None, vec![path]);
        }
        return (path.is_dir().then_some(path), Vec::new());
    }
    let conventional = root.join("commands");
    if conventional.is_dir() {
        return (Some(conventional), Vec::new());
    }
    (None, Vec::new())
}

fn resolve_design_paths(
    root: &Path,
    source: &ResourceSourceConfig,
) -> (Option<PathBuf>, Vec<PathBuf>) {
    if !source.design_paths.is_empty() {
        return (
            None,
            source
                .design_paths
                .iter()
                .filter_map(|path| resolve_design_file(&join_source_child(root, path)))
                .collect(),
        );
    }
    if let Some(designs_path) = source.designs_path.as_deref() {
        let path = join_source_child(root, designs_path);
        if let Some(file) = resolve_design_file(&path) {
            return (None, vec![file]);
        }
        return (path.is_dir().then_some(path), Vec::new());
    }
    if let Some(file) = resolve_design_file(root) {
        return (None, vec![file]);
    }
    let conventional = root.join("designs");
    if conventional.is_dir() {
        return (Some(conventional), Vec::new());
    }
    if has_design_children(root) {
        return (Some(root.to_path_buf()), Vec::new());
    }
    (None, Vec::new())
}

fn resolve_design_file(path: &Path) -> Option<PathBuf> {
    if path.is_file()
        && path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("DESIGN.md"))
    {
        return Some(path.to_path_buf());
    }
    let file = path.join("DESIGN.md");
    file.is_file().then_some(file)
}

fn join_source_child(root: &Path, child: &str) -> PathBuf {
    let child_path = PathBuf::from(child);
    if child_path.is_absolute() {
        child_path
    } else {
        root.join(child_path)
    }
}

fn has_skill_children(path: &Path) -> bool {
    repo::list_dirs(path)
        .into_iter()
        .any(|name| path.join(name).join("SKILL.md").is_file())
}

fn has_design_children(path: &Path) -> bool {
    if !list_design_files(path).is_empty() {
        return true;
    }
    repo::list_dirs(path)
        .into_iter()
        .any(|name| path.join(name).join("DESIGN.md").is_file())
}

fn list_design_files(path: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(path)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            let path = entry.path();
            if file_type.is_file()
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("DESIGN.md"))
            {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    files.sort();
    files
}

fn relative_source_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn pattern_allowed(candidates: &[&str], includes: &[String], excludes: &[String]) -> bool {
    let included = includes.is_empty()
        || includes.iter().any(|pattern| {
            candidates
                .iter()
                .any(|candidate| glob_match(pattern, candidate))
        });
    if !included {
        return false;
    }
    !excludes.iter().any(|pattern| {
        candidates
            .iter()
            .any(|candidate| glob_match(pattern, candidate))
    })
}

fn glob_match(pattern: &str, value: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return false;
    }
    glob_match_bytes(pattern.as_bytes(), value.as_bytes())
}

fn glob_match_bytes(pattern: &[u8], value: &[u8]) -> bool {
    let (mut pattern_index, mut value_index) = (0, 0);
    let mut star_index = None;
    let mut star_value_index = 0;

    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?' || pattern[pattern_index] == value[value_index])
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            star_value_index = value_index;
            pattern_index += 1;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

fn slugify(value: &str) -> String {
    let slug: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "source".into()
    } else {
        trimmed.into()
    }
}

fn stable_hash(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn read_skills(
    repo: &AgentsRepo,
    installs: BTreeMap<String, String>,
    sources: &[LoadedResourceSource],
    warnings: &mut Vec<String>,
) -> Vec<SkillItem> {
    let mut seen = BTreeSet::new();
    let aggregate_skills_dir = runtime_skills_dir(repo);
    let mut items: Vec<SkillItem> = repo::list_dirs(&repo.paths.skills)
        .into_iter()
        .map(|name| {
            let path = Path::new(&repo.paths.skills).join(&name);
            let file = path.join("SKILL.md");
            let text = repo::read_text(&file).unwrap_or_default();
            let frontmatter = repo::parse_frontmatter(&text);
            let item_name = frontmatter.get("name").cloned().unwrap_or(name);
            seen.insert(item_name.clone());
            SkillItem {
                name: item_name,
                description: frontmatter.get("description").cloned().unwrap_or_default(),
                path: repo::display_path(&path),
                file: repo::display_path(&file),
                source_name: "Local".into(),
                source_path: repo.paths.skills.clone(),
                source_kind: "local".into(),
                frontmatter,
                preview: truncate_preview(&text),
                installs: installs.clone(),
            }
        })
        .collect();

    for source in sources {
        let Some(skills_dir) = &source.skills_dir else {
            for skill_dir in &source.skill_dirs {
                read_source_skill_dir(
                    source,
                    skill_dir,
                    &mut seen,
                    warnings,
                    &mut items,
                    &installs,
                    aggregate_skills_dir.as_deref(),
                );
            }
            continue;
        };
        for folder_name in repo::list_dirs(skills_dir) {
            let path = skills_dir.join(&folder_name);
            read_source_skill_dir(
                source,
                &path,
                &mut seen,
                warnings,
                &mut items,
                &installs,
                aggregate_skills_dir.as_deref(),
            );
        }
        for skill_dir in &source.skill_dirs {
            read_source_skill_dir(
                source,
                skill_dir,
                &mut seen,
                warnings,
                &mut items,
                &installs,
                aggregate_skills_dir.as_deref(),
            );
        }
    }

    items
}

fn read_source_skill_dir(
    source: &LoadedResourceSource,
    path: &Path,
    seen: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
    items: &mut Vec<SkillItem>,
    installs: &BTreeMap<String, String>,
    aggregate_skills_dir: Option<&Path>,
) {
    let file = path.join("SKILL.md");
    let text = repo::read_text(&file).unwrap_or_default();
    let frontmatter = repo::parse_frontmatter(&text);
    let folder_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| source.name.clone());
    let item_name = frontmatter
        .get("name")
        .cloned()
        .unwrap_or_else(|| folder_name.clone());
    let relative_path = relative_source_path(&source.resolved_path, path);
    if !pattern_allowed(
        &[&item_name, &folder_name, &relative_path],
        &source.include_skills,
        &source.exclude_skills,
    ) {
        return;
    }
    if !seen.insert(item_name.clone()) {
        warnings.push(format!(
            "Skipped skill `{item_name}` from `{}` because a local or earlier source skill has the same name.",
            source.name
        ));
        return;
    }
    items.push(SkillItem {
        name: item_name,
        description: frontmatter.get("description").cloned().unwrap_or_default(),
        path: repo::display_path(path),
        file: repo::display_path(&file),
        source_name: source.name.clone(),
        source_path: repo::display_path(&source.resolved_path),
        source_kind: source.source_kind.clone(),
        frontmatter,
        preview: truncate_preview(&text),
        installs: source_skill_installs(aggregate_skills_dir, path, installs),
    });
}

fn read_commands(
    repo: &AgentsRepo,
    installs: BTreeMap<String, String>,
    sources: &[LoadedResourceSource],
    warnings: &mut Vec<String>,
) -> Vec<CommandItem> {
    let mut seen = BTreeSet::new();
    let mut items: Vec<CommandItem> = repo::list_markdown_files(&repo.paths.commands)
        .into_iter()
        .map(|name| {
            let file = Path::new(&repo.paths.commands).join(&name);
            let text = repo::read_text(&file).unwrap_or_default();
            let frontmatter = repo::parse_frontmatter(&text);
            let command_name = name.trim_end_matches(".md").to_string();
            seen.insert(command_name.clone());
            CommandItem {
                name: command_name,
                description: frontmatter.get("description").cloned().unwrap_or_default(),
                argument_hint: frontmatter
                    .get("argument-hint")
                    .or_else(|| frontmatter.get("argument_hint"))
                    .cloned()
                    .unwrap_or_default(),
                path: repo::display_path(&file),
                source_name: "Local".into(),
                source_path: repo.paths.commands.clone(),
                source_kind: "local".into(),
                frontmatter,
                preview: truncate_preview(&text),
                installs: installs.clone(),
            }
        })
        .collect();

    for source in sources {
        let Some(commands_dir) = &source.commands_dir else {
            for command_file in &source.command_files {
                read_source_command_file(
                    source,
                    command_file,
                    &mut seen,
                    warnings,
                    &mut items,
                    &installs,
                );
            }
            continue;
        };
        for file_name in repo::list_markdown_files(commands_dir) {
            let file = commands_dir.join(&file_name);
            read_source_command_file(source, &file, &mut seen, warnings, &mut items, &installs);
        }
        for command_file in &source.command_files {
            read_source_command_file(
                source,
                command_file,
                &mut seen,
                warnings,
                &mut items,
                &installs,
            );
        }
    }

    items
}

fn read_source_command_file(
    source: &LoadedResourceSource,
    file: &Path,
    seen: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
    items: &mut Vec<CommandItem>,
    installs: &BTreeMap<String, String>,
) {
    let text = repo::read_text(file).unwrap_or_default();
    let frontmatter = repo::parse_frontmatter(&text);
    let Some(file_name) = file
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    else {
        return;
    };
    let command_name = file_name.trim_end_matches(".md").to_string();
    let relative_path = relative_source_path(&source.resolved_path, file);
    if !pattern_allowed(
        &[&command_name, &relative_path],
        &source.include_commands,
        &source.exclude_commands,
    ) {
        return;
    }
    if !seen.insert(command_name.clone()) {
        warnings.push(format!(
            "Skipped command `/{command_name}` from `{}` because a local or earlier source command has the same name.",
            source.name
        ));
        return;
    }
    items.push(CommandItem {
        name: command_name,
        description: frontmatter.get("description").cloned().unwrap_or_default(),
        argument_hint: frontmatter
            .get("argument-hint")
            .or_else(|| frontmatter.get("argument_hint"))
            .cloned()
            .unwrap_or_default(),
        path: repo::display_path(file),
        source_name: source.name.clone(),
        source_path: repo::display_path(&source.resolved_path),
        source_kind: source.source_kind.clone(),
        frontmatter,
        preview: truncate_preview(&text),
        installs: source_only_installs(installs),
    });
}

fn source_only_installs(installs: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    installs
        .keys()
        .map(|tool| (tool.clone(), "source-only".into()))
        .collect()
}

fn source_skill_installs(
    aggregate_skills_dir: Option<&Path>,
    skill_path: &Path,
    installs: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    if aggregate_skills_dir
        .map(|aggregate| aggregate_contains_skill(aggregate, skill_path))
        .unwrap_or(false)
    {
        return installs.clone();
    }
    source_only_installs(installs)
}

fn aggregate_contains_skill(aggregate_skills_dir: &Path, skill_path: &Path) -> bool {
    let expected = match fs::canonicalize(skill_path) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let entries = match fs::read_dir(aggregate_skills_dir) {
        Ok(entries) => entries,
        Err(_) => return false,
    };
    entries.filter_map(Result::ok).any(|entry| {
        fs::canonicalize(entry.path())
            .map(|path| path == expected)
            .unwrap_or(false)
    })
}

fn read_designs(
    repo: &AgentsRepo,
    sources: &[LoadedResourceSource],
    warnings: &mut Vec<String>,
) -> Vec<DesignItem> {
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    let root = Path::new(&repo.root);
    let root_design = root.join("DESIGN.md");
    if root_design.is_file() {
        read_design_file(
            "Local",
            &PathBuf::from(&repo.root),
            "local",
            &root_design,
            &mut seen,
            warnings,
            &mut items,
            &[],
            &[],
        );
    }
    let designs_dir = Path::new(&repo.paths.designs);
    read_design_dir(
        "Local",
        &PathBuf::from(&repo.root),
        "local",
        designs_dir,
        &mut seen,
        warnings,
        &mut items,
        &[],
        &[],
    );

    for source in sources {
        let Some(designs_dir) = &source.designs_dir else {
            for design_file in &source.design_files {
                read_design_file(
                    &source.name,
                    &source.resolved_path,
                    &source.source_kind,
                    design_file,
                    &mut seen,
                    warnings,
                    &mut items,
                    &source.include_designs,
                    &source.exclude_designs,
                );
            }
            continue;
        };
        read_design_dir(
            &source.name,
            &source.resolved_path,
            &source.source_kind,
            designs_dir,
            &mut seen,
            warnings,
            &mut items,
            &source.include_designs,
            &source.exclude_designs,
        );
        for design_file in &source.design_files {
            read_design_file(
                &source.name,
                &source.resolved_path,
                &source.source_kind,
                design_file,
                &mut seen,
                warnings,
                &mut items,
                &source.include_designs,
                &source.exclude_designs,
            );
        }
    }

    items
}

#[allow(clippy::too_many_arguments)]
fn read_design_dir(
    source_name: &str,
    source_root: &PathBuf,
    source_kind: &str,
    dir: &Path,
    seen: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
    items: &mut Vec<DesignItem>,
    includes: &[String],
    excludes: &[String],
) {
    if !dir.is_dir() {
        return;
    }
    let design = dir.join("DESIGN.md");
    if design.is_file() {
        read_design_file(
            source_name,
            source_root,
            source_kind,
            &design,
            seen,
            warnings,
            items,
            includes,
            excludes,
        );
    }
    for file in list_design_files(dir) {
        if file != design {
            read_design_file(
                source_name,
                source_root,
                source_kind,
                &file,
                seen,
                warnings,
                items,
                includes,
                excludes,
            );
        }
    }
    for name in repo::list_dirs(dir) {
        let file = dir.join(name).join("DESIGN.md");
        if file.is_file() {
            read_design_file(
                source_name,
                source_root,
                source_kind,
                &file,
                seen,
                warnings,
                items,
                includes,
                excludes,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn read_design_file(
    source_name: &str,
    source_root: &PathBuf,
    source_kind: &str,
    file: &Path,
    seen: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
    items: &mut Vec<DesignItem>,
    includes: &[String],
    excludes: &[String],
) {
    let text = repo::read_text(file).unwrap_or_default();
    let frontmatter = repo::parse_frontmatter(&text);
    let parent_name = file
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "DESIGN".into());
    let item_name = frontmatter
        .get("name")
        .cloned()
        .unwrap_or_else(|| parent_name.clone());
    let relative_path = relative_source_path(source_root, file);
    if !pattern_allowed(
        &[&item_name, &parent_name, &relative_path],
        includes,
        excludes,
    ) {
        return;
    }
    let key = item_name.clone();
    if !seen.insert(key.clone()) {
        warnings.push(format!(
            "Skipped design `{key}` from `{source_name}` because a local or earlier source design has the same name."
        ));
        return;
    }
    items.push(DesignItem {
        name: item_name,
        description: frontmatter.get("description").cloned().unwrap_or_default(),
        path: repo::display_path(file),
        file: repo::display_path(file),
        source_name: source_name.into(),
        source_path: repo::display_path(source_root),
        source_kind: source_kind.into(),
        frontmatter,
        preview: truncate_preview(&text),
    });
}

fn read_registry(repo: &AgentsRepo) -> McpRegistry {
    let path = repo.paths.registry.clone();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            return McpRegistry {
                valid: false,
                path,
                error: Some(error.to_string()),
                servers: Vec::new(),
            }
        }
    };

    let parsed: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return McpRegistry {
                valid: false,
                path,
                error: Some(error.to_string()),
                servers: Vec::new(),
            }
        }
    };

    let mut servers = Vec::new();
    if let Some(map) = parsed.get("servers").and_then(Value::as_object) {
        for (name, value) in map {
            let url = value
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let command = value
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let server_type = value
                .get("type")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    if url.is_empty() {
                        "local".into()
                    } else {
                        "remote".into()
                    }
                });
            let targets = value
                .get("targets")
                .and_then(Value::as_object)
                .map(|targets| {
                    targets
                        .iter()
                        .map(|(key, value)| (key.clone(), value.as_bool().unwrap_or(true)))
                        .collect()
                })
                .unwrap_or_default();
            servers.push(McpServerItem {
                name: name.clone(),
                description: value
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                server_type: server_type.clone(),
                transport: value
                    .get("transport")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| {
                        if server_type == "remote" {
                            "http".into()
                        } else {
                            "stdio".into()
                        }
                    }),
                url,
                command,
                args: value
                    .get("args")
                    .and_then(Value::as_array)
                    .map(|args| {
                        args.iter()
                            .filter_map(Value::as_str)
                            .map(ToString::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                has_headers: value.get("headers").is_some(),
                has_environment: value.get("environment").is_some() || value.get("env").is_some(),
                enabled: value
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                targets,
                raw_json: serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
            });
        }
    }
    servers.sort_by(|left, right| left.name.cmp(&right.name));
    McpRegistry {
        valid: true,
        path,
        error: None,
        servers,
    }
}

fn read_tools(repo: &AgentsRepo) -> Vec<ToolStatus> {
    let mut tools = vec![
        link_tool(
            "claude-code",
            "Claude Code",
            BTreeMap::from([
                (
                    "globalInstructions".into(),
                    path_join(&repo.home, &[".claude", "CLAUDE.md"]),
                ),
                (
                    "skills".into(),
                    path_join(&repo.home, &[".claude", "skills"]),
                ),
                (
                    "commands".into(),
                    path_join(&repo.home, &[".claude", "commands"]),
                ),
            ]),
            repo,
        ),
        link_tool(
            "codex",
            "Codex",
            BTreeMap::from([
                (
                    "globalInstructions".into(),
                    path_join(&repo.codex_home, &["AGENTS.md"]),
                ),
                ("skills".into(), path_join(&repo.codex_home, &["skills"])),
                ("commands".into(), path_join(&repo.codex_home, &["prompts"])),
                (
                    "config".into(),
                    path_join(&repo.codex_home, &["config.toml"]),
                ),
            ]),
            repo,
        ),
        link_tool(
            "opencode",
            "OpenCode",
            BTreeMap::from([
                (
                    "globalInstructions".into(),
                    path_join(&repo.home, &[".config", "opencode", "AGENTS.md"]),
                ),
                (
                    "skills".into(),
                    path_join(&repo.home, &[".config", "opencode", "skills"]),
                ),
                (
                    "commands".into(),
                    path_join(&repo.home, &[".config", "opencode", "commands"]),
                ),
                (
                    "config".into(),
                    path_join(&repo.home, &[".config", "opencode", "opencode.json"]),
                ),
            ]),
            repo,
        ),
    ];

    tools.push(copilot_tool(repo));
    tools
}

fn link_tool(
    id: &str,
    label: &str,
    paths: BTreeMap<String, String>,
    repo: &AgentsRepo,
) -> ToolStatus {
    let skills_path = effective_skills_path(repo);
    let resources = BTreeMap::from([
        (
            "globalInstructions".into(),
            symlink_status(paths.get("globalInstructions").unwrap(), &repo.paths.agents),
        ),
        (
            "skills".into(),
            symlink_status(paths.get("skills").unwrap(), &skills_path),
        ),
        (
            "commands".into(),
            not_supported_status(
                paths.get("commands").unwrap(),
                &repo.paths.commands,
                "Command folders are inspectable resources, but the current sync scripts do not install them into this tool.",
            ),
        ),
    ]);
    ToolStatus {
        id: id.into(),
        label: label.into(),
        status: rollup(
            resources
                .values()
                .map(|resource| resource.status.clone())
                .collect(),
        ),
        paths,
        resources,
    }
}

fn effective_skills_path(repo: &AgentsRepo) -> String {
    runtime_skills_dir(repo)
        .map(repo::display_path)
        .unwrap_or_else(|| repo.paths.skills.clone())
}

fn runtime_skills_dir(repo: &AgentsRepo) -> Option<PathBuf> {
    let aggregate = Path::new(&repo.root)
        .join(".agents-manager")
        .join("runtime")
        .join("skills");
    if aggregate.is_dir() {
        Some(aggregate)
    } else {
        None
    }
}

fn copilot_tool(repo: &AgentsRepo) -> ToolStatus {
    let env_path = path_join(
        &repo.home,
        &[".config", "agents", "github-copilot-cli.env.sh"],
    );
    let text = repo::read_text(&env_path).unwrap_or_default();
    let installed = text.contains(&format!(
        "COPILOT_CUSTOM_INSTRUCTIONS_DIRS=\"{}\"",
        repo.agents_home
    )) && text.contains(&format!(
        "COPILOT_SKILLS_DIRS=\"{}/skills\"",
        repo.agents_home
    ));
    let exists = repo::path_exists(&env_path);
    let status = if installed {
        "installed"
    } else if exists {
        "drift"
    } else {
        "missing"
    };
    let resource = InstallStatus {
        status: status.into(),
        target_path: env_path.clone(),
        expected_path: repo.agents_home.clone(),
        actual_path: env_path.clone(),
        message: if installed {
            "Expected env snippet found."
        } else {
            "Env snippet missing or drifted."
        }
        .into(),
    };
    ToolStatus {
        id: "github-copilot-cli".into(),
        label: "GitHub Copilot CLI".into(),
        status: status.into(),
        paths: BTreeMap::from([("envSnippet".into(), env_path)]),
        resources: BTreeMap::from([("envSnippet".into(), resource)]),
    }
}

fn symlink_status(target: &str, expected: &str) -> InstallStatus {
    match fs::symlink_metadata(target) {
        Ok(metadata) => {
            if !metadata.file_type().is_symlink() {
                return InstallStatus {
                    status: "unmanaged".into(),
                    target_path: target.into(),
                    expected_path: expected.into(),
                    actual_path: target.into(),
                    message:
                        "Path exists but is not a symlink managed by the portable agents repo."
                            .into(),
                };
            }
            let actual = fs::read_link(target).unwrap_or_default();
            let target_resolved =
                fs::canonicalize(target).unwrap_or_else(|_| PathBuf::from(target));
            let expected_resolved =
                fs::canonicalize(expected).unwrap_or_else(|_| PathBuf::from(expected));
            let installed = target_resolved == expected_resolved;
            InstallStatus {
                status: if installed { "installed" } else { "drift" }.into(),
                target_path: target.into(),
                expected_path: expected.into(),
                actual_path: repo::display_path(actual),
                message: if installed {
                    "Symlink points at expected repo path."
                } else {
                    "Symlink points somewhere else."
                }
                .into(),
            }
        }
        Err(error) => InstallStatus {
            status: if error.kind() == std::io::ErrorKind::NotFound {
                "missing"
            } else {
                "unknown"
            }
            .into(),
            target_path: target.into(),
            expected_path: expected.into(),
            actual_path: String::new(),
            message: if error.kind() == std::io::ErrorKind::NotFound {
                "Path is missing.".into()
            } else {
                error.to_string()
            },
        },
    }
}

fn not_supported_status(target: &str, expected: &str, message: &str) -> InstallStatus {
    let actual_path = fs::read_link(target)
        .map(repo::display_path)
        .unwrap_or_default();
    InstallStatus {
        status: "not-supported".into(),
        target_path: target.into(),
        expected_path: expected.into(),
        actual_path,
        message: message.into(),
    }
}

fn installs_for(tools: &[ToolStatus], resource_name: &str) -> BTreeMap<String, String> {
    tools
        .iter()
        .filter_map(|tool| {
            tool.resources
                .get(resource_name)
                .map(|resource| (tool.id.clone(), resource.status.clone()))
        })
        .collect()
}

fn read_mcp_statuses(
    repo: &AgentsRepo,
    servers: &[McpServerItem],
) -> BTreeMap<String, Vec<McpInstallStatus>> {
    BTreeMap::from([
        ("claude-code".into(), claude_mcp_statuses(servers)),
        (
            "codex".into(),
            servers
                .iter()
                .map(|server| codex_mcp_status(repo, server))
                .collect(),
        ),
        (
            "opencode".into(),
            servers
                .iter()
                .map(|server| opencode_mcp_status(repo, server))
                .collect(),
        ),
    ])
}

fn codex_mcp_status(repo: &AgentsRepo, server: &McpServerItem) -> McpInstallStatus {
    let config = path_join(&repo.codex_home, &["config.toml"]);
    let Some(text) = repo::read_text(&config) else {
        return mcp_status_with_type(
            "codex",
            &server.name,
            &server.server_type,
            "missing",
            &config,
            "Codex config.toml is missing.",
        );
    };
    let has_table = text.contains(&format!("[mcp_servers.{}]", server.name))
        || text.contains(&format!("[mcp_servers.\"{}\"]", server.name));
    if !has_table {
        return mcp_status_with_type(
            "codex",
            &server.name,
            &server.server_type,
            "missing",
            &config,
            "No matching Codex MCP table found.",
        );
    }
    let expected = if server.server_type == "remote" {
        &server.url
    } else {
        &server.command
    };
    if expected.is_empty() || text.contains(expected) {
        mcp_status_with_type(
            "codex",
            &server.name,
            &server.server_type,
            "installed",
            &config,
            "Matching Codex MCP entry found.",
        )
    } else {
        mcp_status_with_type(
            "codex",
            &server.name,
            &server.server_type,
            "drift",
            &config,
            "Codex MCP entry points elsewhere.",
        )
    }
}

fn opencode_mcp_status(repo: &AgentsRepo, server: &McpServerItem) -> McpInstallStatus {
    let config = path_join(&repo.home, &[".config", "opencode", "opencode.json"]);
    let Some(text) = repo::read_text(&config) else {
        return mcp_status_with_type(
            "opencode",
            &server.name,
            &server.server_type,
            "missing",
            &config,
            "OpenCode config is missing.",
        );
    };
    let cleaned = strip_json_comments(&text);
    let parsed: Value = match serde_json::from_str(&cleaned) {
        Ok(value) => value,
        Err(error) => {
            return mcp_status_with_type(
                "opencode",
                &server.name,
                &server.server_type,
                "invalid",
                &config,
                &error.to_string(),
            )
        }
    };
    let Some(entry) = parsed.get("mcp").and_then(|mcp| mcp.get(&server.name)) else {
        return mcp_status_with_type(
            "opencode",
            &server.name,
            &server.server_type,
            "missing",
            &config,
            "No matching OpenCode MCP entry found.",
        );
    };
    let expected = if server.server_type == "remote" {
        &server.url
    } else {
        &server.command
    };
    let actual = if server.server_type == "remote" {
        entry
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    } else if let Some(command) = entry.get("command").and_then(Value::as_array) {
        command
            .first()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    } else {
        entry
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    if expected.is_empty() || actual == *expected {
        mcp_status_with_type(
            "opencode",
            &server.name,
            &server.server_type,
            "installed",
            &config,
            "Matching OpenCode MCP entry found.",
        )
    } else {
        mcp_status_with_type(
            "opencode",
            &server.name,
            &server.server_type,
            "drift",
            &config,
            "OpenCode MCP entry points elsewhere.",
        )
    }
}

fn claude_mcp_statuses(servers: &[McpServerItem]) -> Vec<McpInstallStatus> {
    if !command_available("claude") {
        return servers
            .iter()
            .map(|server| {
                mcp_status_with_type(
                    "claude-code",
                    &server.name,
                    &server.server_type,
                    "cli-missing",
                    "claude mcp list",
                    "Claude Code CLI is not available on PATH.",
                )
            })
            .collect();
    }
    let output = Command::new("claude").args(["mcp", "list"]).output();
    let Ok(output) = output else {
        return servers
            .iter()
            .map(|server| {
                mcp_status_with_type(
                    "claude-code",
                    &server.name,
                    &server.server_type,
                    "unknown",
                    "claude mcp list",
                    "Unable to read Claude Code MCP list.",
                )
            })
            .collect();
    };
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return servers
            .iter()
            .map(|server| {
                mcp_status_with_type(
                    "claude-code",
                    &server.name,
                    &server.server_type,
                    "unknown",
                    "claude mcp list",
                    &message,
                )
            })
            .collect();
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    servers
        .iter()
        .map(|server| {
            if !text.contains(&server.name) {
                return mcp_status_with_type(
                    "claude-code",
                    &server.name,
                    &server.server_type,
                    "missing",
                    "claude mcp list",
                    "Claude Code MCP list does not include this server.",
                );
            }
            let details = Command::new("claude")
                .args(["mcp", "get", &server.name])
                .output();
            let expected = if server.server_type == "remote" {
                &server.url
            } else {
                &server.command
            };
            if let Ok(details) = details {
                let detail_text = format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&details.stdout),
                    String::from_utf8_lossy(&details.stderr)
                );
                if details.status.success()
                    && !expected.is_empty()
                    && !detail_text.contains(expected)
                {
                    return mcp_status_with_type(
                        "claude-code",
                        &server.name,
                        &server.server_type,
                        "drift",
                        &format!("claude mcp get {}", server.name),
                        "Claude Code MCP entry points elsewhere.",
                    );
                }
            }
            mcp_status_with_type(
                "claude-code",
                &server.name,
                &server.server_type,
                "installed",
                &format!("claude mcp get {}", server.name),
                "Claude Code MCP entry found.",
            )
        })
        .collect()
}

fn mcp_status_with_type(
    tool: &str,
    server: &str,
    server_type: &str,
    status: &str,
    path: &str,
    message: &str,
) -> McpInstallStatus {
    let auth_command = auth::auth_command_for_server(tool, server, server_type);
    let auth_status = if status == "installed" && auth_command.is_some() {
        Some(cached_auth_status(tool))
    } else if status == "cli-missing" {
        Some("cli-missing".into())
    } else if auth_command.is_none() {
        Some("not-supported".into())
    } else {
        Some("unknown".into())
    };
    McpInstallStatus {
        tool: tool.into(),
        server: server.into(),
        status: status.into(),
        path: path.into(),
        message: message.into(),
        auth_status,
        auth_command,
    }
}

fn cached_auth_status(tool: &str) -> String {
    let now = Instant::now();
    let cache = AUTH_STATUS_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    if let Ok(mut guard) = cache.lock() {
        if let Some((checked_at, status)) = guard.get(tool) {
            if now.duration_since(*checked_at) < Duration::from_secs(30) {
                return status.clone();
            }
        }
        let status = auth::check_auth_availability(tool).0;
        guard.insert(tool.into(), (now, status.clone()));
        return status;
    }
    auth::check_auth_availability(tool).0
}

fn truncate_preview(text: &str) -> String {
    const LIMIT: usize = 16_000;
    if text.len() <= LIMIT {
        text.to_string()
    } else {
        format!("{}...\n\n<preview truncated>", &text[..LIMIT])
    }
}

fn command_available(command: &str) -> bool {
    let result = if cfg!(windows) {
        Command::new("where").arg(command).output()
    } else {
        Command::new("sh")
            .args(["-c", &format!("command -v {}", command)])
            .output()
    };
    result
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn rollup(statuses: Vec<String>) -> String {
    let managed: Vec<String> = statuses
        .into_iter()
        .filter(|status| status != "not-supported")
        .collect();
    if managed.is_empty() {
        return "not-supported".into();
    }
    if managed.iter().all(|status| status == "installed") {
        "installed".into()
    } else if managed
        .iter()
        .all(|status| status == "missing" || status == "cli-missing")
    {
        "missing".into()
    } else {
        "drift".into()
    }
}

fn strip_json_comments(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(char) = chars.next() {
        if in_string {
            output.push(char);
            if escaped {
                escaped = false;
            } else if char == '\\' {
                escaped = true;
            } else if char == '"' {
                in_string = false;
            }
            continue;
        }
        if char == '"' {
            in_string = true;
            output.push(char);
            continue;
        }
        if char == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    output.push('\n');
                    break;
                }
            }
            continue;
        }
        output.push(char);
    }
    output.replace(",}", "}").replace(",]", "]")
}

fn path_join(base: &str, segments: &[&str]) -> String {
    let mut path = PathBuf::from(base);
    for segment in segments {
        path.push(segment);
    }
    repo::display_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn build_state_merges_configured_source_skills_and_commands() {
        let root = temp_path("faerry-source-test");
        let repo_root = root.join("repo");
        let source_root = root.join("source");

        fs::create_dir_all(repo_root.join("skills/local")).unwrap();
        fs::create_dir_all(repo_root.join("commands")).unwrap();
        fs::create_dir_all(repo_root.join("mcp")).unwrap();
        fs::create_dir_all(source_root.join("skills/external")).unwrap();
        fs::create_dir_all(source_root.join("commands")).unwrap();

        fs::write(repo_root.join("AGENTS.md"), "# Test Agents\n").unwrap();
        fs::write(repo_root.join("mcp/servers.json"), r#"{"servers":{}}"#).unwrap();
        fs::write(
            repo_root.join("skills/local/SKILL.md"),
            "---\nname: Local Skill\ndescription: Local description\n---\n\nLocal body\n",
        )
        .unwrap();
        fs::write(
            repo_root.join("commands/local.md"),
            "---\ndescription: Local command\n---\n\nLocal command body\n",
        )
        .unwrap();
        fs::write(
            source_root.join("skills/external/SKILL.md"),
            "---\nname: External Skill\ndescription: External description\n---\n\nExternal body\n",
        )
        .unwrap();
        fs::write(
            source_root.join("commands/external.md"),
            "---\ndescription: External command\n---\n\nExternal command body\n",
        )
        .unwrap();
        fs::write(
            repo_root.join(repo::FAERRY_CONFIG_FILENAME),
            format!(
                r#"{{
  "sources": [
    {{
      "name": "external-source",
      "path": "{}",
      "skills": true,
      "commands": true
    }}
  ]
}}"#,
                source_root.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .unwrap();

        let state = build_state(Some(repo::display_path(&repo_root)));

        assert!(state.source_config.exists);
        assert!(state.source_config.valid);
        assert_eq!(state.source_config.sources.len(), 1);
        assert_eq!(state.source_config.sources[0].status, "loaded");

        let external_skill = state
            .skills
            .iter()
            .find(|skill| skill.name == "External Skill")
            .expect("external skill should be merged into app state");
        assert_eq!(external_skill.source_name, "external-source");
        assert_eq!(external_skill.source_kind, "external");
        assert!(external_skill
            .installs
            .values()
            .all(|status| status == "source-only"));

        let external_command = state
            .commands
            .iter()
            .find(|command| command.name == "external")
            .expect("external command should be merged into app state");
        assert_eq!(external_command.source_name, "external-source");
        assert_eq!(external_command.source_kind, "external");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn read_skills_marks_source_skill_installed_when_runtime_aggregate_contains_it() {
        let root = temp_path("faerry-source-install-test");
        let repo_root = root.join("repo");
        let source_root = root.join("source");
        let aggregate_root = repo_root.join(".agents-manager/runtime/skills");

        fs::create_dir_all(repo_root.join("skills/local")).unwrap();
        fs::create_dir_all(source_root.join("skills/external")).unwrap();
        fs::create_dir_all(&aggregate_root).unwrap();
        fs::write(
            repo_root.join("skills/local/SKILL.md"),
            "---\nname: Local Skill\n---\n\nLocal body\n",
        )
        .unwrap();
        fs::write(
            source_root.join("skills/external/SKILL.md"),
            "---\nname: External Skill\n---\n\nExternal body\n",
        )
        .unwrap();
        symlink_dir(
            &source_root.join("skills/external"),
            &aggregate_root.join("external"),
        );

        let repo = AgentsRepo {
            ok: true,
            root: repo::display_path(&repo_root),
            home: repo::display_path(root.join("home")),
            agents_home: repo::display_path(root.join("home/.agents")),
            codex_home: repo::display_path(root.join("home/.codex")),
            paths: RepoPaths {
                agents: repo::display_path(repo_root.join("AGENTS.md")),
                skills: repo::display_path(repo_root.join("skills")),
                commands: repo::display_path(repo_root.join("commands")),
                designs: repo::display_path(repo_root.join("designs")),
                registry: repo::display_path(repo_root.join("mcp/servers.json")),
                scripts: repo::display_path(repo_root.join("scripts")),
            },
        };
        let source = LoadedResourceSource {
            name: "external-source".into(),
            resolved_path: source_root.clone(),
            source_kind: "external".into(),
            skills_dir: Some(source_root.join("skills")),
            skill_dirs: Vec::new(),
            commands_dir: None,
            command_files: Vec::new(),
            designs_dir: None,
            design_files: Vec::new(),
            include_skills: Vec::new(),
            exclude_skills: Vec::new(),
            include_commands: Vec::new(),
            exclude_commands: Vec::new(),
            include_designs: Vec::new(),
            exclude_designs: Vec::new(),
        };
        let installs = BTreeMap::from([
            ("claude-code".into(), "installed".into()),
            ("codex".into(), "installed".into()),
            ("opencode".into(), "installed".into()),
        ]);
        let mut warnings = Vec::new();

        let skills = read_skills(&repo, installs, &[source], &mut warnings);
        let external = skills
            .iter()
            .find(|skill| skill.name == "External Skill")
            .expect("external skill should be loaded");

        assert!(external
            .installs
            .values()
            .all(|status| status == "installed"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn tool_status_ignores_unsupported_command_folder() {
        let root = temp_path("faerry-tool-status-test");
        let repo_root = root.join("repo");
        let home = root.join("home");
        let agents_home = home.join(".agents");
        let claude_home = home.join(".claude");

        fs::create_dir_all(repo_root.join("skills")).unwrap();
        fs::create_dir_all(repo_root.join("commands")).unwrap();
        fs::create_dir_all(&agents_home).unwrap();
        fs::create_dir_all(&claude_home).unwrap();
        fs::write(repo_root.join("AGENTS.md"), "# Test Agents\n").unwrap();
        symlink_file(&repo_root.join("AGENTS.md"), &agents_home.join("AGENTS.md"));
        symlink_dir(&repo_root.join("skills"), &agents_home.join("skills"));
        symlink_file(
            &agents_home.join("AGENTS.md"),
            &claude_home.join("CLAUDE.md"),
        );
        symlink_dir(&agents_home.join("skills"), &claude_home.join("skills"));

        let repo = AgentsRepo {
            ok: true,
            root: repo::display_path(&repo_root),
            home: repo::display_path(&home),
            agents_home: repo::display_path(&agents_home),
            codex_home: repo::display_path(home.join(".codex")),
            paths: RepoPaths {
                agents: repo::display_path(repo_root.join("AGENTS.md")),
                skills: repo::display_path(repo_root.join("skills")),
                commands: repo::display_path(repo_root.join("commands")),
                designs: repo::display_path(repo_root.join("designs")),
                registry: repo::display_path(repo_root.join("mcp/servers.json")),
                scripts: repo::display_path(repo_root.join("scripts")),
            },
        };

        let tools = read_tools(&repo);
        let claude = tools
            .iter()
            .find(|tool| tool.id == "claude-code")
            .expect("Claude Code status should be present");

        assert_eq!(claude.status, "installed");
        assert_eq!(claude.resources["commands"].status, "not-supported");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn build_state_clones_git_source_and_loads_selected_skill_paths() {
        if !command_available("git") {
            return;
        }

        let root = temp_path("faerry-git-source-test");
        let repo_root = root.join("repo");
        let git_root = root.join("git-source");

        fs::create_dir_all(repo_root.join("skills")).unwrap();
        fs::create_dir_all(repo_root.join("commands")).unwrap();
        fs::create_dir_all(repo_root.join("mcp")).unwrap();
        fs::create_dir_all(git_root.join("skills/git-skill")).unwrap();
        fs::write(repo_root.join("AGENTS.md"), "# Test Agents\n").unwrap();
        fs::write(repo_root.join("mcp/servers.json"), r#"{"servers":{}}"#).unwrap();
        fs::write(
            git_root.join("skills/git-skill/SKILL.md"),
            "---\nname: Git Skill\ndescription: From a Git source\n---\n\nGit body\n",
        )
        .unwrap();
        git(&git_root, &["init"]);
        git(&git_root, &["branch", "-M", "main"]);
        git(&git_root, &["add", "."]);
        git(
            &git_root,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "initial",
            ],
        );

        fs::write(
            repo_root.join(repo::FAERRY_CONFIG_FILENAME),
            format!(
                r#"{{
  "sources": [
    {{
      "name": "git-source",
      "url": "{}",
      "ref": "main",
      "skills": true,
      "commands": false,
      "skillPaths": ["skills/git-skill"]
    }}
  ]
}}"#,
                git_root.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .unwrap();

        let state = build_state(Some(repo::display_path(&repo_root)));

        assert_eq!(state.source_config.sources.len(), 1);
        assert_eq!(state.source_config.sources[0].status, "loaded");
        let skill = state
            .skills
            .iter()
            .find(|skill| skill.name == "Git Skill")
            .expect("git skill should be loaded from cloned cache");
        assert_eq!(skill.source_name, "git-source");
        assert_eq!(skill.source_kind, "git");
        assert!(skill.path.contains(".agents-manager"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn include_skill_patterns_filter_source_folder_children() {
        let root = temp_path("faerry-include-pattern-test");
        let repo_root = root.join("repo");
        let source_root = root.join("source");

        fs::create_dir_all(repo_root.join("skills")).unwrap();
        fs::create_dir_all(repo_root.join("commands")).unwrap();
        fs::create_dir_all(repo_root.join("mcp")).unwrap();
        fs::create_dir_all(source_root.join("skills/taste-sweet")).unwrap();
        fs::create_dir_all(source_root.join("skills/planner")).unwrap();
        fs::create_dir_all(source_root.join("skills/skip-me")).unwrap();
        fs::write(repo_root.join("AGENTS.md"), "# Test Agents\n").unwrap();
        fs::write(repo_root.join("mcp/servers.json"), r#"{"servers":{}}"#).unwrap();
        fs::write(
            source_root.join("skills/taste-sweet/SKILL.md"),
            "---\nname: Taste Sweet\ndescription: Included by glob\n---\n\nBody\n",
        )
        .unwrap();
        fs::write(
            source_root.join("skills/planner/SKILL.md"),
            "---\nname: Planner\ndescription: Included by name\n---\n\nBody\n",
        )
        .unwrap();
        fs::write(
            source_root.join("skills/skip-me/SKILL.md"),
            "---\nname: Skip Me\ndescription: Excluded\n---\n\nBody\n",
        )
        .unwrap();
        fs::write(
            repo_root.join(repo::FAERRY_CONFIG_FILENAME),
            format!(
                r#"{{
  "sources": [
    {{
      "name": "filtered-source",
      "path": "{}",
      "skills": true,
      "commands": false,
      "includeSkills": ["taste-*", "Planner"],
      "excludeSkills": ["skip-*"]
    }}
  ]
}}"#,
                source_root.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .unwrap();

        let state = build_state(Some(repo::display_path(&repo_root)));
        let skill_names: BTreeSet<String> = state
            .skills
            .iter()
            .map(|skill| skill.name.clone())
            .collect();

        assert!(skill_names.contains("Taste Sweet"));
        assert!(skill_names.contains("Planner"));
        assert!(!skill_names.contains("Skip Me"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn build_state_loads_designs_from_configured_source() {
        let root = temp_path("faerry-design-source-test");
        let repo_root = root.join("repo");
        let source_root = root.join("source");

        fs::create_dir_all(repo_root.join("skills")).unwrap();
        fs::create_dir_all(repo_root.join("commands")).unwrap();
        fs::create_dir_all(repo_root.join("mcp")).unwrap();
        fs::create_dir_all(source_root.join("examples/mobile")).unwrap();
        fs::create_dir_all(source_root.join("examples/web")).unwrap();
        fs::write(repo_root.join("AGENTS.md"), "# Test Agents\n").unwrap();
        fs::write(repo_root.join("mcp/servers.json"), r#"{"servers":{}}"#).unwrap();
        fs::write(
            source_root.join("examples/mobile/DESIGN.md"),
            "---\nname: Mobile Design\ndescription: Included design\n---\n\n# Mobile\n",
        )
        .unwrap();
        fs::write(
            source_root.join("examples/web/DESIGN.md"),
            "---\nname: Web Design\ndescription: Excluded design\n---\n\n# Web\n",
        )
        .unwrap();
        fs::write(
            repo_root.join(repo::FAERRY_CONFIG_FILENAME),
            format!(
                r#"{{
  "sources": [
    {{
      "name": "design-source",
      "path": "{}",
      "skills": false,
      "commands": false,
      "designs": true,
      "designsPath": "examples",
      "includeDesigns": ["Mobile*"]
    }}
  ]
}}"#,
                source_root.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .unwrap();

        let state = build_state(Some(repo::display_path(&repo_root)));
        let design_names: BTreeSet<String> = state
            .designs
            .iter()
            .map(|design| design.name.clone())
            .collect();

        assert!(design_names.contains("Mobile Design"));
        assert!(!design_names.contains("Web Design"));
        assert_eq!(state.designs[0].source_kind, "external");

        let _ = fs::remove_dir_all(root);
    }

    fn temp_path(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{label}-{}-{timestamp}", std::process::id()))
    }

    fn git(cwd: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[cfg(unix)]
    fn symlink_dir(source: &Path, target: &Path) {
        std::os::unix::fs::symlink(source, target).unwrap();
    }

    #[cfg(unix)]
    fn symlink_file(source: &Path, target: &Path) {
        std::os::unix::fs::symlink(source, target).unwrap();
    }

    #[cfg(windows)]
    fn symlink_dir(source: &Path, target: &Path) {
        std::os::windows::fs::symlink_dir(source, target).unwrap();
    }

    #[cfg(windows)]
    fn symlink_file(source: &Path, target: &Path) {
        std::os::windows::fs::symlink_file(source, target).unwrap();
    }
}
