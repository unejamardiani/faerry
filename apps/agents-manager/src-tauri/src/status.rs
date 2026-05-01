use crate::{models::*, repo};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn build_state(repo_override: Option<String>) -> AppState {
    match repo::detect_repo_with_override(repo_override) {
        Ok(repo) => {
            let registry = read_registry(&repo);
            let tools = read_tools(&repo);
            let skill_installs = installs_for(&tools, "skills");
            let command_installs = installs_for(&tools, "commands");
            let skills = read_skills(&repo, skill_installs);
            let commands = read_commands(&repo, command_installs);
            let mcp_statuses = read_mcp_statuses(&repo, &registry.servers);
            AppState {
                repo: Some(repo),
                repo_error: None,
                generated_at: generated_at(),
                registry,
                tools,
                skills,
                commands,
                mcp_statuses,
            }
        }
        Err(error) => AppState {
            repo: None,
            repo_error: Some(error.to_string()),
            generated_at: generated_at(),
            registry: McpRegistry {
                valid: false,
                path: String::new(),
                error: Some(error.to_string()),
                servers: Vec::new(),
            },
            tools: Vec::new(),
            skills: Vec::new(),
            commands: Vec::new(),
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

fn read_skills(repo: &AgentsRepo, installs: BTreeMap<String, String>) -> Vec<SkillItem> {
    repo::list_dirs(&repo.paths.skills)
        .into_iter()
        .map(|name| {
            let path = Path::new(&repo.paths.skills).join(&name);
            let file = path.join("SKILL.md");
            let frontmatter = repo::read_text(&file)
                .map(|text| repo::parse_frontmatter(&text))
                .unwrap_or_default();
            SkillItem {
                name: frontmatter.get("name").cloned().unwrap_or(name),
                description: frontmatter.get("description").cloned().unwrap_or_default(),
                path: repo::display_path(&path),
                file: repo::display_path(&file),
                installs: installs.clone(),
            }
        })
        .collect()
}

fn read_commands(repo: &AgentsRepo, installs: BTreeMap<String, String>) -> Vec<CommandItem> {
    repo::list_markdown_files(&repo.paths.commands)
        .into_iter()
        .map(|name| {
            let file = Path::new(&repo.paths.commands).join(&name);
            let frontmatter = repo::read_text(&file)
                .map(|text| repo::parse_frontmatter(&text))
                .unwrap_or_default();
            CommandItem {
                name: name.trim_end_matches(".md").to_string(),
                description: frontmatter.get("description").cloned().unwrap_or_default(),
                path: repo::display_path(&file),
                installs: installs.clone(),
            }
        })
        .collect()
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
            let url = value.get("url").and_then(Value::as_str).unwrap_or_default().to_string();
            let command = value.get("command").and_then(Value::as_str).unwrap_or_default().to_string();
            let server_type = value
                .get("type")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| if url.is_empty() { "local".into() } else { "remote".into() });
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
                description: value.get("description").and_then(Value::as_str).unwrap_or_default().to_string(),
                server_type: server_type.clone(),
                transport: value
                    .get("transport")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| if server_type == "remote" { "http".into() } else { "stdio".into() }),
                url,
                command,
                args: value
                    .get("args")
                    .and_then(Value::as_array)
                    .map(|args| args.iter().filter_map(Value::as_str).map(ToString::to_string).collect())
                    .unwrap_or_default(),
                enabled: value.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                targets,
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
                ("globalInstructions".into(), path_join(&repo.home, &[".claude", "CLAUDE.md"])),
                ("skills".into(), path_join(&repo.home, &[".claude", "skills"])),
                ("commands".into(), path_join(&repo.home, &[".claude", "commands"])),
            ]),
            repo,
        ),
        link_tool(
            "codex",
            "Codex",
            BTreeMap::from([
                ("globalInstructions".into(), path_join(&repo.codex_home, &["AGENTS.md"])),
                ("skills".into(), path_join(&repo.codex_home, &["skills"])),
                ("commands".into(), path_join(&repo.codex_home, &["prompts"])),
                ("config".into(), path_join(&repo.codex_home, &["config.toml"])),
            ]),
            repo,
        ),
        link_tool(
            "opencode",
            "OpenCode",
            BTreeMap::from([
                ("globalInstructions".into(), path_join(&repo.home, &[".config", "opencode", "AGENTS.md"])),
                ("skills".into(), path_join(&repo.home, &[".config", "opencode", "skills"])),
                ("commands".into(), path_join(&repo.home, &[".config", "opencode", "commands"])),
                ("config".into(), path_join(&repo.home, &[".config", "opencode", "opencode.json"])),
            ]),
            repo,
        ),
    ];

    tools.push(copilot_tool(repo));
    tools
}

fn link_tool(id: &str, label: &str, paths: BTreeMap<String, String>, repo: &AgentsRepo) -> ToolStatus {
    let resources = BTreeMap::from([
        ("globalInstructions".into(), symlink_status(paths.get("globalInstructions").unwrap(), &repo.paths.agents)),
        ("skills".into(), symlink_status(paths.get("skills").unwrap(), &repo.paths.skills)),
        ("commands".into(), symlink_status(paths.get("commands").unwrap(), &repo.paths.commands)),
    ]);
    ToolStatus {
        id: id.into(),
        label: label.into(),
        status: rollup(resources.values().map(|resource| resource.status.clone()).collect()),
        paths,
        resources,
    }
}

fn copilot_tool(repo: &AgentsRepo) -> ToolStatus {
    let env_path = path_join(&repo.home, &[".config", "agents", "github-copilot-cli.env.sh"]);
    let text = repo::read_text(&env_path).unwrap_or_default();
    let installed = text.contains(&format!("COPILOT_CUSTOM_INSTRUCTIONS_DIRS=\"{}\"", repo.agents_home))
        && text.contains(&format!("COPILOT_SKILLS_DIRS=\"{}/skills\"", repo.agents_home));
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
                    message: "Path exists but is not a symlink managed by the portable agents repo.".into(),
                };
            }
            let actual = fs::read_link(target).unwrap_or_default();
            let target_resolved = fs::canonicalize(target).unwrap_or_else(|_| PathBuf::from(target));
            let expected_resolved = fs::canonicalize(expected).unwrap_or_else(|_| PathBuf::from(expected));
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

fn installs_for(tools: &[ToolStatus], resource_name: &str) -> BTreeMap<String, String> {
    tools
        .iter()
        .filter_map(|tool| tool.resources.get(resource_name).map(|resource| (tool.id.clone(), resource.status.clone())))
        .collect()
}

fn read_mcp_statuses(repo: &AgentsRepo, servers: &[McpServerItem]) -> BTreeMap<String, Vec<McpInstallStatus>> {
    BTreeMap::from([
        ("claude-code".into(), claude_mcp_statuses(servers)),
        ("codex".into(), servers.iter().map(|server| codex_mcp_status(repo, server)).collect()),
        ("opencode".into(), servers.iter().map(|server| opencode_mcp_status(repo, server)).collect()),
    ])
}

fn codex_mcp_status(repo: &AgentsRepo, server: &McpServerItem) -> McpInstallStatus {
    let config = path_join(&repo.codex_home, &["config.toml"]);
    let Some(text) = repo::read_text(&config) else {
        return mcp_status("codex", &server.name, "missing", &config, "Codex config.toml is missing.");
    };
    let has_table = text.contains(&format!("[mcp_servers.{}]", server.name))
        || text.contains(&format!("[mcp_servers.\"{}\"]", server.name));
    if !has_table {
        return mcp_status("codex", &server.name, "missing", &config, "No matching Codex MCP table found.");
    }
    let expected = if server.server_type == "remote" { &server.url } else { &server.command };
    if expected.is_empty() || text.contains(expected) {
        mcp_status("codex", &server.name, "installed", &config, "Matching Codex MCP entry found.")
    } else {
        mcp_status("codex", &server.name, "drift", &config, "Codex MCP entry points elsewhere.")
    }
}

fn opencode_mcp_status(repo: &AgentsRepo, server: &McpServerItem) -> McpInstallStatus {
    let config = path_join(&repo.home, &[".config", "opencode", "opencode.json"]);
    let Some(text) = repo::read_text(&config) else {
        return mcp_status("opencode", &server.name, "missing", &config, "OpenCode config is missing.");
    };
    let cleaned = strip_json_comments(&text);
    let parsed: Value = match serde_json::from_str(&cleaned) {
        Ok(value) => value,
        Err(error) => return mcp_status("opencode", &server.name, "invalid", &config, &error.to_string()),
    };
    let Some(entry) = parsed.get("mcp").and_then(|mcp| mcp.get(&server.name)) else {
        return mcp_status("opencode", &server.name, "missing", &config, "No matching OpenCode MCP entry found.");
    };
    let expected = if server.server_type == "remote" { &server.url } else { &server.command };
    let actual = if server.server_type == "remote" {
        entry.get("url").and_then(Value::as_str).unwrap_or_default().to_string()
    } else if let Some(command) = entry.get("command").and_then(Value::as_array) {
        command.first().and_then(Value::as_str).unwrap_or_default().to_string()
    } else {
        entry.get("command").and_then(Value::as_str).unwrap_or_default().to_string()
    };
    if expected.is_empty() || actual == *expected {
        mcp_status("opencode", &server.name, "installed", &config, "Matching OpenCode MCP entry found.")
    } else {
        mcp_status("opencode", &server.name, "drift", &config, "OpenCode MCP entry points elsewhere.")
    }
}

fn claude_mcp_statuses(servers: &[McpServerItem]) -> Vec<McpInstallStatus> {
    if !command_available("claude") {
        return servers
            .iter()
            .map(|server| mcp_status("claude-code", &server.name, "cli-missing", "claude mcp list", "Claude Code CLI is not available on PATH."))
            .collect();
    }
    let output = Command::new("claude").args(["mcp", "list"]).output();
    let Ok(output) = output else {
        return servers
            .iter()
            .map(|server| mcp_status("claude-code", &server.name, "unknown", "claude mcp list", "Unable to read Claude Code MCP list."))
            .collect();
    };
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return servers
            .iter()
            .map(|server| mcp_status("claude-code", &server.name, "unknown", "claude mcp list", &message))
            .collect();
    }
    let text = format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    servers
        .iter()
        .map(|server| {
            if !text.contains(&server.name) {
                return mcp_status("claude-code", &server.name, "missing", "claude mcp list", "Claude Code MCP list does not include this server.");
            }
            let details = Command::new("claude").args(["mcp", "get", &server.name]).output();
            let expected = if server.server_type == "remote" { &server.url } else { &server.command };
            if let Ok(details) = details {
                let detail_text = format!("{}\n{}", String::from_utf8_lossy(&details.stdout), String::from_utf8_lossy(&details.stderr));
                if details.status.success() && !expected.is_empty() && !detail_text.contains(expected) {
                    return mcp_status("claude-code", &server.name, "drift", &format!("claude mcp get {}", server.name), "Claude Code MCP entry points elsewhere.");
                }
            }
            mcp_status("claude-code", &server.name, "installed", &format!("claude mcp get {}", server.name), "Claude Code MCP entry found.")
        })
        .collect()
}

fn mcp_status(tool: &str, server: &str, status: &str, path: &str, message: &str) -> McpInstallStatus {
    McpInstallStatus {
        tool: tool.into(),
        server: server.into(),
        status: status.into(),
        path: path.into(),
        message: message.into(),
    }
}

fn command_available(command: &str) -> bool {
    let result = if cfg!(windows) {
        Command::new("where").arg(command).output()
    } else {
        Command::new("sh").args(["-c", &format!("command -v {}", command)]).output()
    };
    result.map(|output| output.status.success()).unwrap_or(false)
}

fn rollup(statuses: Vec<String>) -> String {
    if statuses.iter().all(|status| status == "installed") {
        "installed".into()
    } else if statuses.iter().all(|status| status == "missing" || status == "cli-missing") {
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
