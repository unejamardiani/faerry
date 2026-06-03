use crate::models::{RepoValidation, SeveritySummary, ValidationIssue};
use crate::repo;
use std::fs;
use std::path::Path;

pub fn validate_repo(root: &str) -> RepoValidation {
    let path = Path::new(root);
    let mut issues = Vec::new();

    let has_workspace_signal = [
        repo::FAERRY_CONFIG_FILENAME,
        repo::LEGACY_SOURCES_FILENAME,
        "AGENTS.md",
        "skills",
        "agents",
        "commands",
        "designs",
        "DESIGN.md",
        "mcp/servers.json",
    ]
    .iter()
    .any(|entry| path.join(entry).exists());

    if !has_workspace_signal {
        issues.push(ValidationIssue {
            code: "workspace-empty".into(),
            severity: "error".into(),
            path: repo::display_path(path),
            message: "This folder does not contain Faerry workspace content yet.".into(),
            suggestion: "Create faerry.json or add skills, agents, commands, designs, AGENTS.md, or mcp/servers.json.".into(),
        });
    }

    validate_file(
        &mut issues,
        "faerry-json",
        path,
        repo::FAERRY_CONFIG_FILENAME,
        "info",
        false,
        "faerry.json is the preferred workspace configuration file.",
    );
    validate_file(
        &mut issues,
        "agents-md",
        path,
        "AGENTS.md",
        "info",
        false,
        "AGENTS.md is the root global instructions file.",
    );
    validate_dir(
        &mut issues,
        "skills-dir",
        path,
        "skills",
        "info",
        false,
        "Skills directory provides reusable agent skills.",
    );
    validate_dir(
        &mut issues,
        "commands-dir",
        path,
        "commands",
        "info",
        false,
        "Commands directory provides reusable agent prompts.",
    );
    validate_file(
        &mut issues,
        "mcp-registry",
        path,
        "mcp/servers.json",
        "info",
        false,
        "MCP registry is the source of truth for all tool MCP entries.",
    );
    validate_dir(
        &mut issues,
        "templates-dir",
        path,
        "templates",
        "info",
        false,
        "Templates directory provides instruction templates.",
    );
    validate_dir(
        &mut issues,
        "scripts-dir",
        path,
        "scripts",
        "info",
        false,
        "Scripts directory contains the sync scripts.",
    );

    // Validate skills
    let skills_dir = path.join("skills");
    if skills_dir.exists() {
        validate_skills(&mut issues, &skills_dir);
    }

    // Validate commands
    let commands_dir = path.join("commands");
    if commands_dir.exists() {
        validate_commands(&mut issues, &commands_dir);
    }

    // Validate MCP registry
    let registry = path.join("mcp/servers.json");
    if registry.exists() {
        validate_mcp_registry(&mut issues, &registry);
    }

    // Safety: check if path is on OneDrive or cloud
    let path_str = repo::display_path(path);
    if is_cloud_synced_path(&path_str) {
        issues.push(ValidationIssue {
            code: "cloud-path".into(),
            severity: "warning".into(),
            path: path_str.clone(),
            message: "Repository appears to be in a cloud-synced directory (OneDrive, iCloud, Dropbox).".into(),
            suggestion: "Cloud sync may cause issues with symlinks and file watching. Consider a local-only path.".into(),
        });
    }

    let severity_summary = SeveritySummary {
        info: issues.iter().filter(|i| i.severity == "info").count(),
        warning: issues.iter().filter(|i| i.severity == "warning").count(),
        error: issues.iter().filter(|i| i.severity == "error").count(),
    };

    RepoValidation {
        path: path_str,
        issues,
        severity_summary,
    }
}

fn validate_file(
    issues: &mut Vec<ValidationIssue>,
    code: &str,
    root: &Path,
    relative: &str,
    severity: &str,
    required: bool,
    _message: &str,
) {
    let full = root.join(relative);
    if !full.exists() {
        if required {
            issues.push(ValidationIssue {
                code: code.into(),
                severity: severity.into(),
                path: repo::display_path(&full),
                message: format!("Required path is missing: {relative}."),
                suggestion: format!("Create {relative} in the repo root."),
            });
        } else if severity == "warning" {
            issues.push(ValidationIssue {
                code: code.into(),
                severity: "info".into(),
                path: repo::display_path(&full),
                message: format!("Optional path is missing: {relative}."),
                suggestion: String::new(),
            });
        }
    }
}

fn validate_dir(
    issues: &mut Vec<ValidationIssue>,
    code: &str,
    root: &Path,
    relative: &str,
    severity: &str,
    required: bool,
    _message: &str,
) {
    let full = root.join(relative);
    if !full.exists() || !full.is_dir() {
        if required {
            issues.push(ValidationIssue {
                code: code.into(),
                severity: severity.into(),
                path: repo::display_path(&full),
                message: format!("Required directory is missing: {relative}."),
                suggestion: format!("Create {relative}/ in the repo root."),
            });
        }
    }
}

fn validate_skills(issues: &mut Vec<ValidationIssue>, skills_dir: &Path) {
    if let Ok(entries) = fs::read_dir(skills_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let skill_md = entry.path().join("SKILL.md");
            if !skill_md.exists() {
                issues.push(ValidationIssue {
                    code: "skill-missing-skill-md".into(),
                    severity: "warning".into(),
                    path: repo::display_path(&skill_md),
                    message: format!("Skill folder '{name}' is missing SKILL.md."),
                    suggestion: "Add a SKILL.md file with YAML frontmatter to the skill folder."
                        .into(),
                });
                continue;
            }
            if let Some(text) = repo::read_text(&skill_md) {
                let fm = repo::parse_frontmatter(&text);
                if fm.is_empty() {
                    issues.push(ValidationIssue {
                        code: "skill-no-frontmatter".into(),
                        severity: "warning".into(),
                        path: repo::display_path(&skill_md),
                        message: format!("Skill '{name}' SKILL.md has no YAML frontmatter."),
                        suggestion: "Add YAML frontmatter with name and description.".into(),
                    });
                } else if !fm.contains_key("name") {
                    issues.push(ValidationIssue {
                        code: "skill-no-name".into(),
                        severity: "warning".into(),
                        path: repo::display_path(&skill_md),
                        message: format!("Skill '{name}' SKILL.md frontmatter is missing 'name'."),
                        suggestion: "Add a 'name' field to the frontmatter.".into(),
                    });
                }
            }
        }
    }
}

fn validate_commands(issues: &mut Vec<ValidationIssue>, commands_dir: &Path) {
    if let Ok(entries) = fs::read_dir(commands_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            let file = entry.path();
            if let Some(text) = repo::read_text(&file) {
                let fm = repo::parse_frontmatter(&text);
                if fm.is_empty() {
                    issues.push(ValidationIssue {
                        code: "command-no-frontmatter".into(),
                        severity: "info".into(),
                        path: repo::display_path(&file),
                        message: format!("Command '{name}' has no YAML frontmatter."),
                        suggestion: "Add YAML frontmatter with description.".into(),
                    });
                } else if !fm.contains_key("description") {
                    issues.push(ValidationIssue {
                        code: "command-no-description".into(),
                        severity: "warning".into(),
                        path: repo::display_path(&file),
                        message: format!("Command '{name}' frontmatter is missing 'description'."),
                        suggestion: "Add a 'description' field to the frontmatter.".into(),
                    });
                }
            }
        }
    }
}

fn validate_mcp_registry(issues: &mut Vec<ValidationIssue>, registry: &Path) {
    let text = match repo::read_text(registry) {
        Some(t) => t,
        None => {
            issues.push(ValidationIssue {
                code: "mcp-cannot-read".into(),
                severity: "error".into(),
                path: repo::display_path(registry),
                message: "Cannot read MCP registry file.".into(),
                suggestion: "Ensure the file is readable and properly formatted.".into(),
            });
            return;
        }
    };

    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            issues.push(ValidationIssue {
                code: "mcp-invalid-json".into(),
                severity: "error".into(),
                path: repo::display_path(registry),
                message: format!("MCP registry JSON is invalid: {e}."),
                suggestion: "Fix the JSON syntax in mcp/servers.json.".into(),
            });
            return;
        }
    };

    if let Some(servers) = parsed.get("servers").and_then(serde_json::Value::as_object) {
        for (name, server) in servers {
            if let Some(server_type) = server.get("type").and_then(serde_json::Value::as_str) {
                match server_type {
                    "remote" => {
                        if server
                            .get("url")
                            .and_then(serde_json::Value::as_str)
                            .is_none_or(|u| u.is_empty())
                        {
                            issues.push(ValidationIssue {
                                code: "mcp-remote-no-url".into(),
                                severity: "error".into(),
                                path: name.clone(),
                                message: format!("Remote MCP server '{name}' has no URL."),
                                suggestion: "Add a 'url' field to the server entry.".into(),
                            });
                        }
                    }
                    "stdio" | "local" => {
                        if server
                            .get("command")
                            .and_then(serde_json::Value::as_str)
                            .is_none_or(|c| c.is_empty())
                        {
                            issues.push(ValidationIssue {
                                code: "mcp-stdio-no-command".into(),
                                severity: "error".into(),
                                path: name.clone(),
                                message: format!("Stdio MCP server '{name}' has no command."),
                                suggestion: "Add a 'command' field to the server entry.".into(),
                            });
                        }
                    }
                    _ => {
                        issues.push(ValidationIssue {
                            code: "mcp-unknown-type".into(),
                            severity: "warning".into(),
                            path: name.clone(),
                            message: format!(
                                "MCP server '{name}' has unknown type: {server_type}."
                            ),
                            suggestion: "Use 'remote' or 'stdio' as the type.".into(),
                        });
                    }
                }
            }

            // Validate targets if present
            if let Some(targets) = server.get("targets").and_then(serde_json::Value::as_object) {
                let valid_targets = ["claudeCode", "codex", "opencode", "githubCopilotCli"];
                for key in targets.keys() {
                    if !valid_targets.contains(&key.as_str()) {
                        issues.push(ValidationIssue {
                            code: "mcp-unknown-target".into(),
                            severity: "warning".into(),
                            path: name.clone(),
                            message: format!("MCP server '{name}' has unknown target: {key}."),
                            suggestion:
                                "Use one of: claudeCode, codex, opencode, githubCopilotCli.".into(),
                        });
                    }
                }
            }
        }
    }
}

pub fn is_cloud_synced_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("onedrive")
        || lower.contains("icloud")
        || lower.contains("dropbox")
        || lower.contains("google drive")
        || lower.contains("mega")
}
