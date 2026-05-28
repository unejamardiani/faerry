use crate::bundled;
use crate::models::{AgentsRepo, RepoImportPlan, RepoImportResult, ScriptPlan, ScriptResult};
use crate::repo;
use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn plan_action(repo: &AgentsRepo, action: &str) -> Result<ScriptPlan, String> {
    let shared = affected_shared(repo);
    let plan = match action {
        "dryRunMcps" => raw_plan(
            action,
            "Dry Run MCPs",
            repo,
            "bundled:sync-mcps.mjs",
            vec!["--registry", &repo.paths.registry, "--dry-run"],
            vec![
                path_join(&repo.codex_home, &["config.toml"]),
                path_join(&repo.home, &[".config", "opencode", "opencode.json"]),
                "Claude Code user MCP registry via claude CLI".into(),
            ],
            false,
            "Prints intended MCP changes without writing target configs.",
        ),
        "syncMcps" => raw_plan(
            action,
            "Sync MCPs",
            repo,
            "bundled:sync-mcps.mjs",
            vec!["--registry", &repo.paths.registry],
            vec![
                path_join(&repo.codex_home, &["config.toml"]),
                path_join(&repo.home, &[".config", "opencode", "opencode.json"]),
                "Claude Code user MCP registry via claude CLI".into(),
            ],
            true,
            "Preserves unrelated settings and unrelated MCP servers using the repo script.",
        ),
        "linkAgents" => raw_plan(
            action,
            "Run Link Script",
            repo,
            &script_name(repo, "link-agents"),
            vec!["--repo-root", &repo.root],
            shared,
            true,
            "Links global instructions, skills, commands, and the GitHub Copilot CLI env snippet.",
        ),
        "dryRunAll" => raw_plan(
            action,
            "Dry Run All",
            repo,
            &script_name(repo, "sync-all-agents"),
            vec!["--repo-root", &repo.root, "--dry-run-mcps"],
            append_paths(shared, repo),
            true,
            "Current script support only dry-runs the MCP portion. Link and package steps still run inside sync-all-agents.",
        ),
        "syncAll" => raw_plan(
            action,
            "Sync All",
            repo,
            &script_name(repo, "sync-all-agents"),
            vec!["--repo-root", &repo.root, "--with-mcps"],
            append_paths(shared, repo),
            true,
            "Runs the repo orchestrator, including linking, packaging, and MCP sync.",
        ),
        "syncClaudeCode" => raw_plan(
            action,
            "Sync Tool: Claude Code",
            repo,
            "bundled:sync-mcps.mjs",
            vec!["--registry", &repo.paths.registry, "--target", "claude-code"],
            vec![path_join(&repo.home, &[".claude"])],
            false,
            "Uses the Claude Code CLI for repo-managed MCP entries.",
        ),
        "syncCodex" => raw_plan(
            action,
            "Sync Tool: Codex",
            repo,
            "bundled:sync-mcps.mjs",
            vec!["--registry", &repo.paths.registry, "--target", "codex"],
            vec![path_join(&repo.codex_home, &["config.toml"])],
            true,
            "Updates repo-managed Codex MCP tables and preserves unrelated Codex settings.",
        ),
        "syncOpenCode" => raw_plan(
            action,
            "Sync Tool: OpenCode",
            repo,
            "bundled:sync-mcps.mjs",
            vec!["--registry", &repo.paths.registry, "--target", "opencode"],
            vec![path_join(&repo.home, &[".config", "opencode", "opencode.json"])],
            true,
            "Updates repo-managed OpenCode MCP entries and preserves unrelated OpenCode settings.",
        ),
        _ => return Err(format!("Unknown action: {action}")),
    };
    Ok(plan)
}

pub fn run_plan(plan: &ScriptPlan) -> Result<ScriptResult, String> {
    let mut command = resolve_command(plan)?;
    command.current_dir(&plan.cwd);
    command.env("PORTABLE_AGENTS_REPO_ROOT", &plan.cwd);
    let output = command.output().map_err(|error| error.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(ScriptResult {
        ok: output.status.success(),
        exit_code: output.status.code(),
        backups: extract_backups(&format!("{stdout}\n{stderr}")),
        stdout,
        stderr,
    })
}

pub fn command_available(command: &str) -> bool {
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

pub fn open_path(target: &str) -> Result<(), String> {
    if target.trim().is_empty() {
        return Err("Path is empty.".into());
    }
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(target);
        command
    } else if cfg!(windows) {
        let mut command = Command::new("explorer");
        command.arg(target);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(target);
        command
    };
    command.spawn().map_err(|error| error.to_string())?;
    Ok(())
}

pub fn choose_repo_path() -> Result<Option<String>, String> {
    if cfg!(target_os = "macos") {
        choose_repo_path_macos()
    } else if cfg!(windows) {
        choose_repo_path_windows()
    } else {
        choose_repo_path_linux()
    }
}

pub fn plan_repo_import(
    source: &str,
    destination: &str,
    branch: Option<&str>,
    shallow: bool,
) -> Result<RepoImportPlan, String> {
    let source = source.trim();
    let destination = repo::resolve_user_path(destination)?;
    validate_import_inputs(source, &destination)?;
    let source_type = import_source_type(source);
    let branch = normalized_branch(branch);
    let display_command = match source_type.as_str() {
        "zip-url" => format!(
            "curl -L --fail {} -o <temp>.zip; extract <temp>.zip -> {}",
            shell_arg(source),
            shell_arg(&repo::display_path(&destination))
        ),
        "zip-file" => format!(
            "extract {} -> {}",
            shell_arg(source),
            shell_arg(&repo::display_path(&destination))
        ),
        _ => {
            let mut parts = vec!["git clone".to_string()];
            if shallow {
                parts.push("--depth 1".into());
            }
            if let Some(branch) = &branch {
                parts.push(format!("--branch {}", shell_arg(branch)));
            }
            parts.push(shell_arg(source));
            parts.push(shell_arg(&repo::display_path(&destination)));
            parts.join(" ")
        }
    };

    Ok(RepoImportPlan {
        source: source.into(),
        destination: repo::display_path(&destination),
        source_type,
        branch,
        shallow,
        display_command,
        affected_paths: vec![repo::display_path(&destination)],
        note: "Destination must not already exist. ZIP imports are extracted into a temporary folder, then normalized so the selected destination becomes the repo root.".into(),
    })
}

pub fn run_repo_import(
    source: &str,
    destination: &str,
    branch: Option<&str>,
    shallow: bool,
) -> Result<RepoImportResult, String> {
    let plan = plan_repo_import(source, destination, branch, shallow)?;
    let destination = PathBuf::from(&plan.destination);
    let parent = destination
        .parent()
        .ok_or_else(|| "Destination has no parent directory.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;

    match plan.source_type.as_str() {
        "git" => run_git_import(&plan, &destination),
        "zip-url" | "zip-file" => run_zip_import(&plan, &destination),
        _ => Err(format!(
            "Unsupported import source type: {}",
            plan.source_type
        )),
    }
}

fn raw_plan(
    action: &str,
    title: &str,
    repo: &AgentsRepo,
    command: &str,
    args: Vec<&str>,
    affected_paths: Vec<String>,
    backups_may_be_created: bool,
    note: &str,
) -> ScriptPlan {
    let args: Vec<String> = args.into_iter().map(ToString::to_string).collect();
    ScriptPlan {
        action: action.into(),
        title: title.into(),
        cwd: repo.root.clone(),
        command: command.into(),
        display_command: shell_command(command, &args),
        args,
        affected_paths,
        backups_may_be_created,
        note: note.into(),
    }
}

fn validate_import_inputs(source: &str, destination: &Path) -> Result<(), String> {
    if source.is_empty() {
        return Err("Source is required.".into());
    }
    if destination.as_os_str().is_empty() {
        return Err("Destination is required.".into());
    }
    if destination.exists() {
        return Err(format!(
            "Destination already exists: {}",
            repo::display_path(destination)
        ));
    }
    if import_source_type(source) == "zip-file" {
        let source_path = repo::resolve_user_path(source)?;
        if !source_path.exists() {
            return Err(format!(
                "ZIP file does not exist: {}",
                repo::display_path(source_path)
            ));
        }
    }
    Ok(())
}

fn import_source_type(source: &str) -> String {
    let lower = source.to_ascii_lowercase();
    let path_part = lower.split('?').next().unwrap_or(&lower);
    let is_url = lower.starts_with("http://") || lower.starts_with("https://");
    if path_part.ends_with(".zip") && is_url {
        "zip-url".into()
    } else if path_part.ends_with(".zip") {
        "zip-file".into()
    } else {
        "git".into()
    }
}

fn normalized_branch(branch: Option<&str>) -> Option<String> {
    branch
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn run_git_import(plan: &RepoImportPlan, destination: &Path) -> Result<RepoImportResult, String> {
    let mut command = Command::new("git");
    command.arg("clone");
    if plan.shallow {
        command.args(["--depth", "1"]);
    }
    if let Some(branch) = &plan.branch {
        command.args(["--branch", branch]);
    }
    let output = command
        .args([&plan.source, &plan.destination])
        .output()
        .map_err(|error| error.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let valid = repo::is_agents_repo(destination);
    if output.status.success() && !valid {
        stderr.push_str("\nClone completed, but the destination does not match the expected portable agents repo layout.");
    }
    Ok(RepoImportResult {
        ok: output.status.success() && valid,
        destination: plan.destination.clone(),
        repo_path: valid.then(|| plan.destination.clone()),
        stdout,
        stderr,
    })
}

fn run_zip_import(plan: &RepoImportPlan, destination: &Path) -> Result<RepoImportResult, String> {
    let temp_root = std::env::temp_dir().join(format!(
        "agents-manager-import-{}-{}",
        std::process::id(),
        timestamp()
    ));
    let extract_dir = temp_root.join("extract");
    fs::create_dir_all(&extract_dir).map_err(|error| error.to_string())?;
    let zip_path = if plan.source_type == "zip-url" {
        let zip_path = temp_root.join("download.zip");
        download_zip(&plan.source, &zip_path)?;
        zip_path
    } else {
        repo::resolve_user_path(&plan.source)?
    };

    let extract_output = extract_zip(&zip_path, &extract_dir)?;
    let Some(repo_root) = find_extracted_repo_root(&extract_dir) else {
        let _ = fs::remove_dir_all(&temp_root);
        return Ok(RepoImportResult {
            ok: false,
            destination: plan.destination.clone(),
            repo_path: None,
            stdout: extract_output.0,
            stderr: format!(
                "{}\nZIP extracted, but no portable agents repo layout was found.",
                extract_output.1
            ),
        });
    };

    move_dir(&repo_root, destination).map_err(|error| error.to_string())?;
    let _ = fs::remove_dir_all(&temp_root);
    let valid = repo::is_agents_repo(destination);
    Ok(RepoImportResult {
        ok: valid,
        destination: plan.destination.clone(),
        repo_path: valid.then(|| plan.destination.clone()),
        stdout: extract_output.0,
        stderr: extract_output.1,
    })
}

fn download_zip(source: &str, destination: &Path) -> Result<(), String> {
    let output = if cfg!(windows) {
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri {} -OutFile {}",
                    powershell_quote(source),
                    powershell_quote(&repo::display_path(destination))
                ),
            ])
            .output()
    } else {
        Command::new("curl")
            .args([
                "-L",
                "--fail",
                source,
                "-o",
                &repo::display_path(destination),
            ])
            .output()
    }
    .map_err(|error| error.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn extract_zip(zip_path: &Path, destination: &Path) -> Result<(String, String), String> {
    let output = if cfg!(windows) {
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Expand-Archive -LiteralPath {} -DestinationPath {} -Force",
                    powershell_quote(&repo::display_path(zip_path)),
                    powershell_quote(&repo::display_path(destination))
                ),
            ])
            .output()
    } else if cfg!(target_os = "macos") && command_exists("ditto") {
        Command::new("ditto")
            .args([
                "-x",
                "-k",
                &repo::display_path(zip_path),
                &repo::display_path(destination),
            ])
            .output()
    } else {
        Command::new("unzip")
            .args([
                "-q",
                &repo::display_path(zip_path),
                "-d",
                &repo::display_path(destination),
            ])
            .output()
    }
    .map_err(|error| error.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        Ok((stdout, stderr))
    } else {
        Err(stderr)
    }
}

fn find_extracted_repo_root(root: &Path) -> Option<PathBuf> {
    if repo::is_agents_repo(root) {
        return Some(root.to_path_buf());
    }
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && repo::is_agents_repo(&path) {
            return Some(path);
        }
    }
    None
}

fn move_dir(source: &Path, destination: &Path) -> io::Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_dir_recursive(source, destination)?;
            fs::remove_dir_all(source)
        }
    }
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {command}")])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn resolve_command(plan: &ScriptPlan) -> Result<Command, String> {
    if let Some(script_name) = bundled::display_name(&plan.command) {
        if !bundled::bundled_script_exists(script_name) {
            return Err(format!("Bundled script not found: {script_name}"));
        }
        let script_path = bundled::script_path(script_name)?;
        if script_name.ends_with(".mjs") {
            let mut command = Command::new("node");
            command.arg(script_path);
            command.args(&plan.args);
            return Ok(command);
        }
        if script_name.ends_with(".ps1") {
            let mut command = Command::new("powershell.exe");
            command.args([
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &bundled::display_path(script_path),
            ]);
            command.args(&plan.args);
            return Ok(command);
        }
        let mut command = Command::new(script_path);
        command.args(&plan.args);
        return Ok(command);
    }
    if plan.command == "node" {
        let mut command = Command::new("node");
        command.args(&plan.args);
        return Ok(command);
    }
    if plan.command.ends_with(".ps1") {
        let mut command = Command::new("powershell.exe");
        command.args(["-ExecutionPolicy", "Bypass", "-File", &plan.command]);
        command.args(&plan.args);
        return Ok(command);
    }
    if let Some(rest) = plan.command.strip_prefix("./") {
        let mut command = Command::new(Path::new(&plan.cwd).join(rest));
        command.args(&plan.args);
        return Ok(command);
    }
    let mut command = Command::new(&plan.command);
    command.args(&plan.args);
    Ok(command)
}

fn script_name(_repo: &AgentsRepo, base: &str) -> String {
    if cfg!(windows) {
        return format!("bundled:{base}.ps1");
    }
    format!("bundled:{base}.sh")
}

fn affected_shared(repo: &AgentsRepo) -> Vec<String> {
    vec![
        repo.agents_home.clone(),
        path_join(&repo.home, &[".claude"]),
        repo.codex_home.clone(),
        path_join(&repo.home, &[".config", "opencode"]),
        path_join(
            &repo.home,
            &[".config", "agents", "github-copilot-cli.env.sh"],
        ),
    ]
}

fn append_paths(mut paths: Vec<String>, repo: &AgentsRepo) -> Vec<String> {
    paths.push(path_join(&repo.codex_home, &["config.toml"]));
    paths.push(path_join(
        &repo.home,
        &[".config", "opencode", "opencode.json"],
    ));
    paths
}

fn path_join(base: &str, segments: &[&str]) -> String {
    let mut path = PathBuf::from(base);
    for segment in segments {
        path.push(segment);
    }
    repo::display_path(path)
}

fn shell_command(command: &str, args: &[String]) -> String {
    std::iter::once(bundled::bundled_command_label(command))
        .chain(args.iter().cloned())
        .map(|value| shell_arg(&value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "_./:=@-".contains(ch))
    {
        value.into()
    } else {
        format!("{value:?}")
    }
}

fn extract_backups(output: &str) -> Vec<String> {
    let mut backups = BTreeSet::new();
    for line in output.lines() {
        if let Some((_, rest)) = line.split_once("Backups:") {
            let value = rest.trim();
            if !value.is_empty() && value != "none" {
                backups.insert(value.to_string());
            }
        }
        if let Some((_, rest)) = line.split_once(" -> ") {
            if line.contains("backed up ") {
                backups.insert(rest.trim().to_string());
            }
        }
    }
    backups.into_iter().collect()
}

fn choose_repo_path_macos() -> Result<Option<String>, String> {
    let output = Command::new("osascript")
        .args([
            "-e",
            r#"POSIX path of (choose folder with prompt "Choose portable agents repo")"#,
        ])
        .output()
        .map_err(|error| error.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("User canceled") || stderr.contains("-128") {
            return Ok(None);
        }
        return Err(stderr.trim().to_string());
    }

    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}

fn choose_repo_path_windows() -> Result<Option<String>, String> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = 'Choose portable agents repo'
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  Write-Output $dialog.SelectedPath
}
"#;
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", script])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| error.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(selected))
    }
}

fn choose_repo_path_linux() -> Result<Option<String>, String> {
    let candidates = [
        (
            "zenity",
            vec![
                "--file-selection",
                "--directory",
                "--title=Choose portable agents repo",
            ],
        ),
        ("kdialog", vec!["--getexistingdirectory", "."]),
    ];

    for (command, args) in candidates {
        let available = Command::new("sh")
            .args(["-c", &format!("command -v {command}")])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if !available {
            continue;
        }
        let output = Command::new(command)
            .args(args)
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok((!selected.is_empty()).then_some(selected));
        }
        return Ok(None);
    }

    Err(
        "No folder picker found. Install zenity or kdialog, or use AGENTS_REPO for repo override."
            .into(),
    )
}
