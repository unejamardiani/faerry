use crate::{
    bundled,
    models::{AgentsRepo, DiffPreview, DiffSection},
    repo, scripts,
};
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    process::Command,
};

const MANAGED_START: &str = "# BEGIN portable-agents managed MCP servers";
const MANAGED_END: &str = "# END portable-agents managed MCP servers";

#[derive(Debug, Clone)]
struct RegistryServer {
    name: String,
    server_type: String,
    url: String,
    command: String,
    args: Vec<String>,
    enabled: bool,
    targets: BTreeMap<String, bool>,
    headers: Option<BTreeMap<String, String>>,
    environment: Option<BTreeMap<String, String>>,
}

pub fn preview_action(repo: &AgentsRepo, action: &str) -> Result<DiffPreview, String> {
    let plan = scripts::plan_action(repo, action)?;
    let mut sections = Vec::new();

    if includes_link_preview(action) {
        sections.extend(link_preview_sections(repo));
    }

    if includes_mcp_preview(action) {
        let servers = read_registry(repo)?;
        if includes_claude_mcp_preview(action) {
            sections.push(claude_mcp_dry_run_section(repo, action));
        }
        if includes_codex_mcp_preview(action) {
            sections.push(codex_mcp_diff_section(repo, &servers)?);
        }
        if includes_opencode_mcp_preview(action) {
            sections.push(opencode_mcp_diff_section(repo, &servers)?);
        }
    }

    if sections.is_empty() {
        sections.push(DiffSection {
            title: "No Preview Available".into(),
            path: plan.cwd.clone(),
            section_type: "info".into(),
            status: "info".into(),
            diff: "This action does not have a native diff preview yet. Review the command and affected paths before running.".into(),
        });
    }

    Ok(DiffPreview {
        action: action.into(),
        title: format!("Preview: {}", plan.title),
        sections,
    })
}

fn includes_link_preview(action: &str) -> bool {
    matches!(action, "linkAgents" | "dryRunAll" | "syncAll")
}

fn includes_mcp_preview(action: &str) -> bool {
    matches!(
        action,
        "dryRunMcps" | "syncMcps" | "dryRunAll" | "syncAll" | "syncClaudeCode" | "syncCodex" | "syncOpenCode"
    )
}

fn includes_claude_mcp_preview(action: &str) -> bool {
    matches!(action, "dryRunMcps" | "syncMcps" | "dryRunAll" | "syncAll" | "syncClaudeCode")
}

fn includes_codex_mcp_preview(action: &str) -> bool {
    matches!(action, "dryRunMcps" | "syncMcps" | "dryRunAll" | "syncAll" | "syncCodex")
}

fn includes_opencode_mcp_preview(action: &str) -> bool {
    matches!(action, "dryRunMcps" | "syncMcps" | "dryRunAll" | "syncAll" | "syncOpenCode")
}

fn link_preview_sections(repo: &AgentsRepo) -> Vec<DiffSection> {
    let mut sections = Vec::new();

    if repo.root != repo.agents_home {
        sections.push(symlink_section("Shared AGENTS.md", path_join(&repo.agents_home, &["AGENTS.md"]), repo.paths.agents.clone()));
        sections.push(symlink_section("Shared skills", path_join(&repo.agents_home, &["skills"]), repo.paths.skills.clone()));
        sections.push(symlink_section("Shared commands", path_join(&repo.agents_home, &["commands"]), repo.paths.commands.clone()));
        sections.push(symlink_section(
            "Shared templates",
            path_join(&repo.agents_home, &["templates"]),
            path_join(&repo.root, &["templates"]),
        ));
    }

    sections.push(symlink_section(
        "Claude Code global instructions",
        path_join(&repo.home, &[".claude", "CLAUDE.md"]),
        path_join(&repo.agents_home, &["AGENTS.md"]),
    ));
    sections.push(symlink_section(
        "Claude Code skills",
        path_join(&repo.home, &[".claude", "skills"]),
        path_join(&repo.agents_home, &["skills"]),
    ));
    sections.push(symlink_section(
        "Claude Code commands",
        path_join(&repo.home, &[".claude", "commands"]),
        path_join(&repo.agents_home, &["commands"]),
    ));

    sections.push(symlink_section(
        "Codex global instructions",
        path_join(&repo.codex_home, &["AGENTS.md"]),
        path_join(&repo.agents_home, &["AGENTS.md"]),
    ));
    sections.push(symlink_section(
        "Codex skills",
        path_join(&repo.codex_home, &["skills"]),
        path_join(&repo.agents_home, &["skills"]),
    ));
    sections.push(symlink_section(
        "Codex prompts",
        path_join(&repo.codex_home, &["prompts"]),
        path_join(&repo.agents_home, &["commands"]),
    ));

    sections.push(symlink_section(
        "OpenCode global instructions",
        path_join(&repo.home, &[".config", "opencode", "AGENTS.md"]),
        path_join(&repo.agents_home, &["AGENTS.md"]),
    ));
    sections.push(symlink_section(
        "OpenCode skills",
        path_join(&repo.home, &[".config", "opencode", "skills"]),
        path_join(&repo.agents_home, &["skills"]),
    ));
    sections.push(symlink_section(
        "OpenCode commands",
        path_join(&repo.home, &[".config", "opencode", "commands"]),
        path_join(&repo.agents_home, &["commands"]),
    ));

    let copilot_path = path_join(&repo.home, &[".config", "agents", "github-copilot-cli.env.sh"]);
    let expected = format!(
        "export COPILOT_CUSTOM_INSTRUCTIONS_DIRS=\"{}\"\nexport COPILOT_SKILLS_DIRS=\"{}/skills\"\n",
        repo.agents_home, repo.agents_home
    );
    let current = fs::read_to_string(&copilot_path).unwrap_or_default();
    sections.push(text_diff_section("GitHub Copilot CLI env snippet", &copilot_path, &current, &expected));

    sections
}

fn symlink_section(title: &str, target: String, expected: String) -> DiffSection {
    let current = current_path_state(&target);
    let expected_text = format!("symlink -> {expected}");
    let status = if current == expected_text { "unchanged" } else { "changed" };
    DiffSection {
        title: title.into(),
        path: target.clone(),
        section_type: "symlink".into(),
        status: status.into(),
        diff: unified_diff(&target, &current, &expected_text),
    }
}

fn current_path_state(path: &str) -> String {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                let target = fs::read_link(path)
                    .map(repo::display_path)
                    .unwrap_or_else(|error| format!("unreadable symlink: {error}"));
                format!("symlink -> {target}")
            } else if metadata.is_dir() {
                "directory (will be backed up/replaced by script if needed)".into()
            } else {
                "file (will be backed up/replaced by script if needed)".into()
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "missing".into(),
        Err(error) => format!("unknown: {error}"),
    }
}

fn read_registry(repo: &AgentsRepo) -> Result<Vec<RegistryServer>, String> {
    let text = fs::read_to_string(&repo.paths.registry).map_err(|error| error.to_string())?;
    let value: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let mut servers = Vec::new();
    if let Some(map) = value.get("servers").and_then(Value::as_object) {
        for (name, server) in map {
            let url = server.get("url").and_then(Value::as_str).unwrap_or_default().to_string();
            let command = server.get("command").and_then(Value::as_str).unwrap_or_default().to_string();
            let server_type = server
                .get("type")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| if url.is_empty() { "local".into() } else { "remote".into() });
            servers.push(RegistryServer {
                name: name.clone(),
                server_type,
                url,
                command,
                args: server
                    .get("args")
                    .and_then(Value::as_array)
                    .map(|args| args.iter().filter_map(Value::as_str).map(ToString::to_string).collect())
                    .unwrap_or_default(),
                enabled: server.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                targets: object_to_bool_map(server.get("targets")),
                headers: object_to_string_map(server.get("headers")),
                environment: object_to_string_map(server.get("environment")),
            });
        }
    }
    servers.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(servers)
}

fn object_to_bool_map(value: Option<&Value>) -> BTreeMap<String, bool> {
    value
        .and_then(Value::as_object)
        .map(|map| map.iter().map(|(key, value)| (key.clone(), value.as_bool().unwrap_or(true))).collect())
        .unwrap_or_default()
}

fn object_to_string_map(value: Option<&Value>) -> Option<BTreeMap<String, String>> {
    value.and_then(Value::as_object).map(|map| {
        map.iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    value.as_str().map(ToString::to_string).unwrap_or_else(|| value.to_string()),
                )
            })
            .collect()
    })
}

fn target_enabled(server: &RegistryServer, target: &str) -> bool {
    server.targets.get(target).copied().unwrap_or(true)
}

fn claude_mcp_dry_run_section(repo: &AgentsRepo, action: &str) -> DiffSection {
    let script_path = match bundled::script_path("sync-mcps.mjs") {
        Ok(path) => path,
        Err(error) => {
            return DiffSection {
                title: "Claude Code MCP dry-run".into(),
                path: "claude mcp".into(),
                section_type: "command-output".into(),
                status: "error".into(),
                diff: error,
            }
        }
    };
    let mut command = Command::new("node");
    command.arg(script_path);
    command.args(["--registry", &repo.paths.registry, "--dry-run"]);
    if action == "syncClaudeCode" {
        command.args(["--target", "claude-code"]);
    }
    command.current_dir(&repo.root);
    command.env("PORTABLE_AGENTS_REPO_ROOT", &repo.root);
    let output = command.output();
    match output {
        Ok(output) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            DiffSection {
                title: "Claude Code MCP dry-run".into(),
                path: "claude mcp".into(),
                section_type: "command-output".into(),
                status: if output.status.success() { "info" } else { "error" }.into(),
                diff: text,
            }
        }
        Err(error) => DiffSection {
            title: "Claude Code MCP dry-run".into(),
            path: "claude mcp".into(),
            section_type: "command-output".into(),
            status: "error".into(),
            diff: error.to_string(),
        },
    }
}

fn codex_mcp_diff_section(repo: &AgentsRepo, servers: &[RegistryServer]) -> Result<DiffSection, String> {
    let path = path_join(&repo.codex_home, &["config.toml"]);
    let current = fs::read_to_string(&path).unwrap_or_default();
    let managed_names: Vec<String> = servers
        .iter()
        .filter(|server| target_enabled(server, "codex"))
        .map(|server| server.name.clone())
        .collect();
    let next = format!("{}{}", remove_managed_toml(&current, &managed_names), codex_toml_block(servers));
    Ok(text_diff_section("Codex MCP config", &path, &current, &next))
}

fn codex_toml_block(servers: &[RegistryServer]) -> String {
    let mut lines = vec![MANAGED_START.to_string(), "# Source: ~/.agents/mcp/servers.json".into()];
    for server in servers.iter().filter(|server| target_enabled(server, "codex")) {
        lines.push(String::new());
        lines.push(format!("[mcp_servers.{}]", toml_key(&server.name)));
        if server.server_type == "remote" {
            lines.push(format!("url = {}", toml_string(&server.url)));
            if let Some(headers) = &server.headers {
                lines.push(format!("http_headers = {}", toml_inline_object(headers)));
            }
        } else {
            lines.push(format!("command = {}", toml_string(&server.command)));
            if !server.args.is_empty() {
                lines.push(format!("args = [{}]", server.args.iter().map(|arg| toml_string(arg)).collect::<Vec<_>>().join(", ")));
            }
            if let Some(environment) = &server.environment {
                lines.push(format!("env = {}", toml_inline_object(environment)));
            }
        }
        lines.push(format!("enabled = {}", if server.enabled { "true" } else { "false" }));
    }
    lines.push(String::new());
    lines.push(MANAGED_END.into());
    lines.push(String::new());
    lines.join("\n")
}

fn remove_managed_toml(text: &str, server_names: &[String]) -> String {
    let without_marker = remove_marker_block(text, MANAGED_START, MANAGED_END);
    let without_tables = remove_codex_tables(&without_marker, server_names);
    let trimmed = without_tables.trim_end();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n\n")
    }
}

fn remove_marker_block(text: &str, start: &str, end: &str) -> String {
    let mut output = String::new();
    let mut dropping = false;
    for line in text.lines() {
        if line.trim() == start {
            dropping = true;
            continue;
        }
        if dropping && line.trim() == end {
            dropping = false;
            continue;
        }
        if !dropping {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn remove_codex_tables(text: &str, server_names: &[String]) -> String {
    let mut output = String::new();
    let mut dropping = false;
    for line in text.lines() {
        if let Some(table) = toml_table_name(line) {
            dropping = server_names.iter().any(|name| {
                table == format!("mcp_servers.{name}")
                    || table.starts_with(&format!("mcp_servers.{name}."))
                    || table == format!("mcp_servers.{}", serde_json::to_string(name).unwrap_or_default())
                    || table.starts_with(&format!("mcp_servers.{}.", serde_json::to_string(name).unwrap_or_default()))
            });
        }
        if !dropping {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn toml_table_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') && !trimmed.starts_with("[[") {
        Some(trimmed.trim_start_matches('[').trim_end_matches(']').trim().to_string())
    } else {
        None
    }
}

fn opencode_mcp_diff_section(repo: &AgentsRepo, servers: &[RegistryServer]) -> Result<DiffSection, String> {
    let path = path_join(&repo.home, &[".config", "opencode", "opencode.json"]);
    let current = fs::read_to_string(&path).unwrap_or_default();
    let mut config: Value = if current.trim().is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_str(&strip_json_comments(&current)).map_err(|error| format!("OpenCode config is invalid JSON: {error}"))?
    };

    let object = config.as_object_mut().ok_or_else(|| "OpenCode config root must be a JSON object.".to_string())?;
    object.entry("$schema").or_insert_with(|| Value::String("https://opencode.ai/config.json".into()));
    if !object.get("mcp").is_some_and(Value::is_object) {
        object.insert("mcp".into(), Value::Object(Map::new()));
    }
    let mcp = object.get_mut("mcp").and_then(Value::as_object_mut).unwrap();
    for server in servers.iter().filter(|server| target_enabled(server, "opencode")) {
        mcp.insert(server.name.clone(), opencode_server(server));
    }

    let next = format!("{}\n", serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?);
    Ok(text_diff_section("OpenCode MCP config", &path, &current, &next))
}

fn opencode_server(server: &RegistryServer) -> Value {
    let mut object = Map::new();
    if server.server_type == "remote" {
        object.insert("type".into(), Value::String("remote".into()));
        object.insert("url".into(), Value::String(server.url.clone()));
        object.insert("enabled".into(), Value::Bool(server.enabled));
        if let Some(headers) = &server.headers {
            object.insert("headers".into(), string_map_value(headers));
        }
    } else {
        object.insert("type".into(), Value::String("local".into()));
        object.insert(
            "command".into(),
            Value::Array(std::iter::once(server.command.clone()).chain(server.args.iter().cloned()).map(Value::String).collect()),
        );
        object.insert("enabled".into(), Value::Bool(server.enabled));
        if let Some(environment) = &server.environment {
            object.insert("environment".into(), string_map_value(environment));
        }
    }
    Value::Object(object)
}

fn string_map_value(map: &BTreeMap<String, String>) -> Value {
    Value::Object(map.iter().map(|(key, value)| (key.clone(), Value::String(value.clone()))).collect())
}

fn text_diff_section(title: &str, path: &str, current: &str, expected: &str) -> DiffSection {
    DiffSection {
        title: title.into(),
        path: path.into(),
        section_type: "text".into(),
        status: if normalize_newlines(current) == normalize_newlines(expected) {
            "unchanged"
        } else {
            "changed"
        }
        .into(),
        diff: unified_diff(path, current, expected),
    }
}

fn unified_diff(path: &str, current: &str, expected: &str) -> String {
    let current = normalize_newlines(current);
    let expected = normalize_newlines(expected);
    if current == expected {
        return "No changes.".into();
    }

    let old: Vec<&str> = current.lines().collect();
    let new: Vec<&str> = expected.lines().collect();
    let mut table = vec![vec![0usize; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            table[i][j] = if old[i] == new[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }

    let mut lines = vec![format!("--- current:{path}"), format!("+++ expected:{path}"), "@@".into()];
    let (mut i, mut j) = (0, 0);
    while i < old.len() && j < new.len() {
        if old[i] == new[j] {
            lines.push(format!(" {}", old[i]));
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            lines.push(format!("-{}", old[i]));
            i += 1;
        } else {
            lines.push(format!("+{}", new[j]));
            j += 1;
        }
    }
    while i < old.len() {
        lines.push(format!("-{}", old[i]));
        i += 1;
    }
    while j < new.len() {
        lines.push(format!("+{}", new[j]));
        j += 1;
    }
    lines.join("\n")
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
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

fn toml_key(name: &str) -> String {
    if name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
        name.into()
    } else {
        toml_string(name)
    }
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

fn toml_inline_object(values: &BTreeMap<String, String>) -> String {
    let entries = values
        .iter()
        .map(|(key, value)| format!("{} = {}", toml_string(key), toml_string(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {entries} }}")
}

fn path_join(base: &str, segments: &[&str]) -> String {
    let mut path = PathBuf::from(base);
    for segment in segments {
        path.push(segment);
    }
    repo::display_path(path)
}
