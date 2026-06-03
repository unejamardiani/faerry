mod auth;
mod bundled;
mod logs;
mod mcp_editor;
mod models;
mod preview;
mod profiles;
mod repo;
mod script_versions;
mod scripts;
mod source_editor;
mod status;
mod validation;

use models::{
    AgentsMdInfo, AppState, CreateResult, DiffPreview, LogEntry, McpRegistryEditResult,
    McpServerFormData, PackageArtifact, PackageResult, Profile, RepoImportPlan, RepoImportResult,
    RepoValidation, RuntimeInfo, ScriptPlan, ScriptResult, ScriptVersionInfo, SelectiveSyncPlan,
    SourceConfigEditResult, SourceFormData, StructuredOutput, UpdateCheckResult,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

#[tauri::command]
async fn get_state(repo_path: Option<String>) -> Result<AppState, String> {
    tauri::async_runtime::spawn_blocking(move || status::build_state(repo_path))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn plan_action(action: String, repo_path: Option<String>) -> Result<ScriptPlan, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|error| error.to_string())?;
    scripts::plan_action(&repo, &action)
}

#[tauri::command]
async fn run_action(action: String, repo_path: Option<String>) -> Result<ScriptResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let repo = repo::detect_repo_with_override(repo_path).map_err(|error| error.to_string())?;
        let plan = scripts::plan_action(&repo, &action)?;
        scripts::run_plan(&plan)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn preview_action(action: String, repo_path: Option<String>) -> Result<DiffPreview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let repo = repo::detect_repo_with_override(repo_path).map_err(|error| error.to_string())?;
        preview::preview_action(&repo, &action)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn choose_repo_path() -> Result<Option<String>, String> {
    scripts::choose_repo_path()
}

#[tauri::command]
fn prepare_workspace(repo_path: String) -> Result<CreateResult, String> {
    let root = Path::new(&repo_path);
    if !root.exists() {
        fs::create_dir_all(root).map_err(|error| error.to_string())?;
    }
    if !root.is_dir() {
        return Ok(CreateResult {
            ok: false,
            path: repo::display_path(root),
            message: "Selected path is not a folder.".into(),
        });
    }
    let path = repo::standard_source_config_path(root);
    if path.exists() {
        return Ok(CreateResult {
            ok: true,
            path: repo::display_path(path),
            message: "Workspace is already prepared.".into(),
        });
    }
    fs::write(&path, "{\n  \"sources\": []\n}\n").map_err(|error| error.to_string())?;
    Ok(CreateResult {
        ok: true,
        path: repo::display_path(path),
        message: "Workspace prepared with faerry.json.".into(),
    })
}

#[tauri::command]
fn plan_repo_import(
    source: String,
    destination: String,
    branch: Option<String>,
    shallow: Option<bool>,
) -> Result<RepoImportPlan, String> {
    scripts::plan_repo_import(
        &source,
        &destination,
        branch.as_deref(),
        shallow.unwrap_or(false),
    )
}

#[tauri::command]
async fn run_repo_import(
    source: String,
    destination: String,
    branch: Option<String>,
    shallow: Option<bool>,
) -> Result<RepoImportResult, String> {
    scripts::run_repo_import(
        &source,
        &destination,
        branch.as_deref(),
        shallow.unwrap_or(false),
    )
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    scripts::open_path(&path)
}

// --- Feature 1: Runtime/About Panel ---

#[tauri::command]
fn get_runtime_info(repo_path: Option<String>) -> Result<RuntimeInfo, String> {
    let path_clone = repo_path.clone();
    let repo = repo::detect_repo_with_override(repo_path).map_err(|e| e.to_string())?;
    let scripts = script_versions::get_bundled_scripts();
    let repo_has_scripts = repo::path_exists(&repo.paths.scripts);

    let platform = if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else {
        "Linux"
    };
    let arch = std::env::consts::ARCH;
    let script_family = if cfg!(windows) { ".ps1" } else { ".sh" };

    let dependencies = get_dependencies();

    // AGENTS.md info
    let agents_path = Path::new(&repo.paths.agents);
    let agents_exists = agents_path.exists();
    let agents_size = agents_path.metadata().map(|m| m.len()).unwrap_or(0);
    let agents_modified = agents_path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|d| {
            let secs = d.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            secs.to_string()
        });

    let repo_mode = if path_clone.is_some() {
        "selected"
    } else if env::var("AGENTS_REPO").is_ok() || env::var("AGENTS_HOME").is_ok() {
        "env override"
    } else if repo::is_agents_repo(&repo.root) {
        "auto-detected"
    } else {
        "fallback"
    };

    Ok(RuntimeInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        repo_root: repo.root,
        repo_mode: repo_mode.to_string(),
        bundled_script_count: scripts.len(),
        scripts,
        repo_has_local_scripts: repo_has_scripts,
        platform: platform.to_string(),
        platform_arch: arch.to_string(),
        script_family: script_family.to_string(),
        dependencies,
        agents_md_exists: agents_exists,
        agents_md_size: agents_size,
        agents_md_modified: agents_modified,
    })
}

fn get_dependencies() -> BTreeMap<String, String> {
    let mut deps = BTreeMap::new();
    for cmd in &["node", "git", "curl", "claude", "codex", "opencode"] {
        deps.insert(
            cmd.to_string(),
            if scripts::command_available(cmd) {
                "available"
            } else {
                "missing"
            }
            .to_string(),
        );
    }
    // Archive tools
    if cfg!(target_os = "macos") {
        deps.insert(
            "ditto".into(),
            if scripts::command_available("ditto") {
                "available"
            } else {
                "missing"
            }
            .into(),
        );
    } else if cfg!(target_os = "windows") {
        deps.insert("powershell".into(), "available".into());
    } else {
        deps.insert(
            "unzip".into(),
            if scripts::command_available("unzip") {
                "available"
            } else {
                "missing"
            }
            .into(),
        );
    }
    deps
}

// --- Feature 4: Script Versions ---

#[tauri::command]
fn get_script_versions(repo_path: Option<String>) -> Result<Vec<ScriptVersionInfo>, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|e| e.to_string())?;
    Ok(script_versions::get_script_versions(&repo.root))
}

// --- Feature 6: Structured Script Output ---

#[tauri::command]
fn parse_script_output(stdout: String, stderr: String, exit_code: Option<i32>) -> StructuredOutput {
    let output = format!("{stdout}\n{stderr}");
    let mut summary = Vec::new();
    let mut changed = Vec::new();
    let mut skipped = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut backups = Vec::new();
    let mut auth_hints = Vec::new();

    for line in output.lines() {
        let lower = line.to_lowercase();
        if is_error_line(&lower) {
            errors.push(line.to_string());
        } else if is_warning_line(&lower) {
            warnings.push(line.to_string());
        } else if lower.contains("skipped") || lower.contains("skip") {
            skipped.push(line.to_string());
        } else if lower.contains("changed") || lower.contains("modified") {
            changed.push(line.to_string());
        } else if lower.contains("backup") {
            backups.push(line.to_string());
        } else if lower.contains("auth") || lower.contains("login") || lower.contains("credential")
        {
            auth_hints.push(line.to_string());
        } else if line.trim().is_empty() {
            continue;
        } else if summary.len() < 10 {
            summary.push(line.to_string());
        }
    }

    StructuredOutput {
        summary,
        changed,
        skipped,
        warnings,
        errors,
        backups,
        auth_hints,
        raw_stdout: stdout,
        raw_stderr: stderr,
        exit_code,
    }
}

fn is_error_line(lower: &str) -> bool {
    let value = lower.trim();
    if value.is_empty() || is_archive_listing_line(value) {
        return false;
    }
    value.starts_with("error")
        || value.starts_with("fatal")
        || value.starts_with("failed")
        || value.starts_with("npm err")
        || value.contains(" error:")
        || value.contains(" failed")
        || value.contains(" failed:")
        || value.contains("failure")
}

fn is_warning_line(lower: &str) -> bool {
    let value = lower.trim();
    if value.is_empty() || is_archive_listing_line(value) {
        return false;
    }
    value.starts_with("warning")
        || value.starts_with("warn")
        || value.contains(" warning:")
        || value.contains(" warn:")
}

fn is_archive_listing_line(value: &str) -> bool {
    value.contains("node_modules/")
        || value.starts_with("archive contents")
        || value.starts_with("archive details")
        || value.starts_with("ignored (")
}

// --- Feature 15: Repo Validation ---

#[tauri::command]
fn validate_repo(repo_path: Option<String>) -> Result<RepoValidation, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|e| e.to_string())?;
    Ok(validation::validate_repo(&repo.root))
}

// --- Feature 16: AGENTS.md Preview ---

#[tauri::command]
fn get_agents_md_info(repo_path: Option<String>) -> Result<AgentsMdInfo, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|e| e.to_string())?;
    let path = Path::new(&repo.paths.agents);
    let exists = path.exists();

    if !exists {
        return Ok(AgentsMdInfo {
            path: repo::display_path(path),
            exists: false,
            size: 0,
            last_modified: None,
            content: String::new(),
            valid: false,
            issues: vec!["AGENTS.md file does not exist.".into()],
        });
    }

    let metadata = path.metadata().map_err(|e| e.to_string())?;
    let size = metadata.len();
    let last_modified = metadata
        .modified()
        .ok()
        .and_then(|d| d.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_secs().to_string());

    let content = repo::read_text(path).unwrap_or_default();
    let mut issues = Vec::new();
    let valid = !content.trim().is_empty();
    if !valid {
        issues.push("AGENTS.md exists but is empty.".into());
    }

    let has_frontmatter = content.starts_with("---\n");
    if has_frontmatter {
        let fm = repo::parse_frontmatter(&content);
        if fm.is_empty() || (!fm.contains_key("name") && !fm.contains_key("description")) {
            issues.push("AGENTS.md may lack frontmatter with name/description.".into());
        }
    }

    Ok(AgentsMdInfo {
        path: repo::display_path(path),
        exists: true,
        size,
        last_modified,
        content,
        valid,
        issues,
    })
}

// --- Feature 21: Local Logs ---

#[tauri::command]
fn get_logs() -> Result<Vec<LogEntry>, String> {
    Ok(logs::list_logs())
}

#[tauri::command]
fn clear_logs_cmd() -> Result<(), String> {
    logs::clear_logs();
    Ok(())
}

#[tauri::command]
fn log_action(
    action: String,
    repo_path: String,
    command: String,
    ok: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    backups: Vec<String>,
) {
    let entry = logs::log_from_script_result(
        &action, &repo_path, &command, ok, exit_code, &stdout, &stderr, &backups,
    );
    logs::append_log(entry);
}

// --- Feature 10: MCP Registry Editor ---

#[tauri::command]
fn validate_mcp_server(data: McpServerFormData) -> Result<Vec<String>, String> {
    Ok(mcp_editor::validate_server(&data))
}

#[tauri::command]
fn edit_mcp_server(
    registry_path: String,
    name: String,
    action: String,
    data: Option<McpServerFormData>,
) -> Result<McpRegistryEditResult, String> {
    Ok(mcp_editor::edit_server(
        &registry_path,
        &name,
        &action,
        data.as_ref(),
    ))
}

#[tauri::command]
fn validate_source_form(data: SourceFormData) -> Result<Vec<String>, String> {
    Ok(source_editor::validate_source(&data))
}

#[tauri::command]
fn edit_source_config(
    repo_path: Option<String>,
    action: String,
    index: Option<usize>,
    data: Option<SourceFormData>,
) -> Result<SourceConfigEditResult, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|e| e.to_string())?;
    Ok(source_editor::edit_source_config(
        &repo.root,
        &action,
        index,
        data.as_ref(),
    ))
}

#[tauri::command]
fn migrate_sources_json(repo_path: Option<String>) -> Result<SourceConfigEditResult, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|e| e.to_string())?;
    Ok(source_editor::migrate_sources_json(&repo.root))
}

// --- Feature 11: Skill Creation Wizard ---

#[tauri::command]
fn create_skill(
    repo_path: String,
    folder_name: String,
    skill_name: String,
    description: String,
    body: String,
) -> Result<CreateResult, String> {
    // Validate name is filesystem-safe
    if !folder_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Ok(CreateResult {
            ok: false,
            path: String::new(),
            message:
                "Folder name must contain only alphanumeric characters, hyphens, and underscores."
                    .into(),
        });
    }
    if description.is_empty() {
        return Ok(CreateResult {
            ok: false,
            path: String::new(),
            message: "Description is required.".into(),
        });
    }

    let skill_dir = Path::new(&repo_path).join("skills").join(&folder_name);
    if skill_dir.exists() {
        return Ok(CreateResult {
            ok: false,
            path: String::new(),
            message: format!(
                "Skill folder already exists: {}",
                repo::display_path(&skill_dir)
            ),
        });
    }

    let skill_md = skill_dir.join("SKILL.md");
    let frontmatter = format!("---\nname: {skill_name}\ndescription: {description}\n---\n\n{body}");
    fs::create_dir_all(&skill_dir).map_err(|e| e.to_string())?;
    fs::write(&skill_md, frontmatter).map_err(|e| e.to_string())?;

    Ok(CreateResult {
        ok: true,
        path: repo::display_path(&skill_md),
        message: format!("Skill created at {}", folder_name),
    })
}

// --- Feature 12: Command Creation Wizard ---

#[tauri::command]
fn create_command(
    repo_path: String,
    command_name: String,
    description: String,
    argument_hint: String,
    body: String,
) -> Result<CreateResult, String> {
    if !command_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Ok(CreateResult {
            ok: false,
            path: String::new(),
            message:
                "Command name must contain only alphanumeric characters, hyphens, and underscores."
                    .into(),
        });
    }
    if description.is_empty() {
        return Ok(CreateResult {
            ok: false,
            path: String::new(),
            message: "Description is required.".into(),
        });
    }

    let file_path = Path::new(&repo_path)
        .join("commands")
        .join(format!("{command_name}.md"));
    if file_path.exists() {
        return Ok(CreateResult {
            ok: false,
            path: String::new(),
            message: format!(
                "Command file already exists: {}",
                repo::display_path(&file_path)
            ),
        });
    }

    let frontmatter = if argument_hint.is_empty() {
        format!("---\ndescription: {description}\n---\n\n{body}")
    } else {
        format!("---\ndescription: {description}\nargument-hint: {argument_hint}\n---\n\n{body}")
    };

    fs::write(&file_path, frontmatter).map_err(|e| e.to_string())?;

    Ok(CreateResult {
        ok: true,
        path: repo::display_path(&file_path),
        message: format!("Command created: /{}", command_name),
    })
}

// --- Feature 13: Profiles ---

#[tauri::command]
fn get_profiles(repo_path: Option<String>) -> Result<Vec<Profile>, String> {
    // Try to read from repo if available
    if let Some(repo) = repo_path {
        let profiles_file = Path::new(&repo).join("profiles.json");
        if profiles_file.exists() {
            if let Ok(text) = fs::read_to_string(&profiles_file) {
                if let Ok(profiles) = serde_json::from_str::<Vec<Profile>>(&text) {
                    return Ok(profiles);
                }
            }
        }
    }

    Ok(profiles::default_profiles())
}

#[tauri::command]
fn save_profiles(repo_path: String, profiles: Vec<Profile>) -> Result<CreateResult, String> {
    let path = Path::new(&repo_path).join("profiles.json");
    let text = serde_json::to_string_pretty(&profiles).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(CreateResult {
        ok: true,
        path: repo::display_path(&path),
        message: "Profiles saved.".into(),
    })
}

// --- Feature 19: Packaging ---

#[tauri::command]
fn package_claude_skills(repo_path: Option<String>) -> Result<PackageResult, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|e| e.to_string())?;
    let script = if cfg!(windows) {
        "package-claude-skills.ps1"
    } else {
        "package-claude-skills.sh"
    };
    execute_package_script(&repo.root, script)
}

fn execute_package_script(repo_path: &str, script_name: &str) -> Result<PackageResult, String> {
    let script_path = bundled::script_path(script_name)?;
    let mut cmd = if script_name.ends_with(".ps1") {
        let mut cmd = Command::new("powershell.exe");
        cmd.args([
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &bundled::display_path(script_path),
        ]);
        cmd
    } else {
        Command::new(script_path)
    };

    cmd.current_dir(repo_path)
        .env("PORTABLE_AGENTS_REPO_ROOT", repo_path);
    let output = cmd.output().map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Collect artifact paths from output
    let mut artifacts = Vec::new();
    for line in stdout.lines() {
        if let Some((_, path_str)) = line.split_once("Artifact: ") {
            let path_str = path_str.trim();
            let path_buf = Path::new(path_str);
            if !path_str.is_empty() && path_buf.exists() {
                let size = path_buf.metadata().map(|m| m.len()).unwrap_or(0);
                let name = path_buf
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                artifacts.push(PackageArtifact {
                    name,
                    path: repo::display_path(path_buf),
                    size,
                });
            }
        }
    }

    Ok(PackageResult {
        ok: output.status.success(),
        message: if output.status.success() {
            "Packaging completed successfully."
        } else {
            "Packaging completed with errors."
        }
        .into(),
        artifacts,
        stdout,
        stderr,
    })
}

// --- Feature 20: Update Check ---

#[tauri::command]
fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    // TODO: In production, check GitHub releases or a version manifest endpoint
    // For now, return up-to-date status
    Ok(UpdateCheckResult {
        current_version: current_version.clone(),
        latest_version: None,
        update_url: Some("https://github.com/unejamardiani/faerry/releases".into()),
        up_to_date: true,
        note: format!("Update check requires a remote endpoint. Current: v{current_version}."),
    })
}

// --- Feature 22: Safety Guardrails ---

#[tauri::command]
fn check_safety_guards(repo_path: Option<String>) -> Result<Vec<String>, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|e| e.to_string())?;
    let mut warnings = Vec::new();

    // Check if repo path is in a cloud-synced directory
    let path_str = &repo.root;
    if validation::is_cloud_synced_path(path_str) {
        warnings.push(format!(
            "Repository is in a cloud-synced directory. This may cause symlink issues: {path_str}"
        ));
    }

    // Check for OneDrive specifically (common issue on Windows)
    let home = Path::new(&repo.home);
    if home.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .to_lowercase()
            .contains("onedrive")
    }) {
        warnings.push("OneDrive detected in home directory. Symlinks may break.".into());
    }

    // Check if target paths exist as non-symlinks
    let target_paths: Vec<(&str, PathBuf)> = vec![
        ("claude-code", Path::new(&repo.home).join(".claude")),
        ("codex", PathBuf::from(&repo.codex_home)),
        (
            "opencode",
            Path::new(&repo.home).join(".config").join("opencode"),
        ),
    ];
    for (tool_id, target_dir) in &target_paths {
        if target_dir.exists() && !target_dir.is_symlink() {
            warnings.push(format!("{tool_id} target directory exists but is not a symlink. Apply may overwrite contents."));
        }
    }

    // Check for unmanaged MCP entries
    let registry = Path::new(&repo.paths.registry);
    if let Some(text) = repo::read_text(registry) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
            if let Some(servers) = parsed.get("servers").and_then(Value::as_object) {
                for (name, server) in servers {
                    let enabled = server
                        .get("enabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(true);
                    if !enabled {
                        warnings.push(format!(
                            "MCP server '{name}' is currently disabled in the registry."
                        ));
                    }
                }
            }
        }
    }

    Ok(warnings)
}

#[tauri::command]
fn plan_selective_sync(
    repo_path: Option<String>,
    tools: Vec<String>,
    categories: Vec<String>,
) -> Result<SelectiveSyncPlan, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|e| e.to_string())?;
    let mut plans = Vec::new();
    let mut warnings = Vec::new();

    let wants_links = categories
        .iter()
        .any(|category| ["globals", "skills", "commands"].contains(&category.as_str()));
    let wants_mcps = categories.iter().any(|category| category == "mcps");

    if wants_links {
        warnings.push("Selective link sync needs script-level skip flags. Current bundled script can only link globals, skills, commands, and Copilot env together.".into());
        plans.push(scripts::plan_action(&repo, "linkAgents")?);
    }

    if wants_mcps {
        for tool in tools {
            match tool.as_str() {
                "claude-code" => plans.push(scripts::plan_action(&repo, "syncClaudeCode")?),
                "codex" => plans.push(scripts::plan_action(&repo, "syncCodex")?),
                "opencode" => plans.push(scripts::plan_action(&repo, "syncOpenCode")?),
                "github-copilot-cli" => {
                    warnings.push("GitHub Copilot CLI has no MCP sync target.".into())
                }
                other => warnings.push(format!("Unknown selective sync tool: {other}.")),
            }
        }
    }

    Ok(SelectiveSyncPlan {
        title: "Selective Sync Plan".into(),
        supported: warnings.is_empty() || plans.iter().any(|plan| plan.action.starts_with("sync")),
        plans,
        warnings,
    })
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_state,
            plan_action,
            run_action,
            preview_action,
            choose_repo_path,
            prepare_workspace,
            plan_repo_import,
            run_repo_import,
            open_path,
            // Feature 1: Runtime/About
            get_runtime_info,
            // Feature 4: Script Versions
            get_script_versions,
            // Feature 6: Structured Output
            parse_script_output,
            // Feature 15: Repo Validation
            validate_repo,
            // Feature 16: AGENTS.md Preview
            get_agents_md_info,
            // Feature 21: Local Logs
            get_logs,
            clear_logs_cmd,
            log_action,
            // Feature 10: MCP Registry Editor
            validate_mcp_server,
            edit_mcp_server,
            validate_source_form,
            edit_source_config,
            migrate_sources_json,
            // Feature 11: Skill Creation
            create_skill,
            // Feature 12: Command Creation
            create_command,
            // Feature 13: Profiles
            get_profiles,
            save_profiles,
            // Feature 19: Packaging
            package_claude_skills,
            // Feature 20: Update Check
            check_for_updates,
            // Feature 22: Safety Guardrails
            check_safety_guards,
            // Feature 14: Selective Sync
            plan_selective_sync,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Faerry");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_output_does_not_treat_archive_paths_as_errors() {
        let output = parse_script_output(
            [
                "Archive Contents",
                "   1.8kB node_modules/es-errors/CHANGELOG.md",
                "   4.0kB node_modules/http-errors/HISTORY.md",
                "Done.",
            ]
            .join("\n"),
            String::new(),
            Some(0),
        );

        assert!(output.errors.is_empty());
        assert_eq!(output.exit_code, Some(0));
    }

    #[test]
    fn structured_output_still_reports_real_errors() {
        let output = parse_script_output(
            "error: unable to write target\nfailed to sync MCP\n".into(),
            String::new(),
            Some(1),
        );

        assert_eq!(output.errors.len(), 2);
    }
}
