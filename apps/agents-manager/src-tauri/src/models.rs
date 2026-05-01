use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentsRepo {
    pub ok: bool,
    pub root: String,
    pub home: String,
    pub agents_home: String,
    pub codex_home: String,
    pub paths: RepoPaths,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoPaths {
    pub agents: String,
    pub skills: String,
    pub commands: String,
    pub registry: String,
    pub scripts: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub repo: Option<AgentsRepo>,
    pub repo_error: Option<String>,
    pub generated_at: String,
    pub registry: McpRegistry,
    pub tools: Vec<ToolStatus>,
    pub skills: Vec<SkillItem>,
    pub commands: Vec<CommandItem>,
    pub mcp_statuses: BTreeMap<String, Vec<McpInstallStatus>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillItem {
    pub name: String,
    pub description: String,
    pub path: String,
    pub file: String,
    pub installs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandItem {
    pub name: String,
    pub description: String,
    pub path: String,
    pub installs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRegistry {
    pub valid: bool,
    pub path: String,
    pub error: Option<String>,
    pub servers: Vec<McpServerItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerItem {
    pub name: String,
    pub description: String,
    pub server_type: String,
    pub transport: String,
    pub url: String,
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
    pub targets: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStatus {
    pub status: String,
    pub target_path: String,
    pub expected_path: String,
    pub actual_path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub id: String,
    pub label: String,
    pub status: String,
    pub paths: BTreeMap<String, String>,
    pub resources: BTreeMap<String, InstallStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInstallStatus {
    pub tool: String,
    pub server: String,
    pub status: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptPlan {
    pub action: String,
    pub title: String,
    pub cwd: String,
    pub command: String,
    pub args: Vec<String>,
    pub affected_paths: Vec<String>,
    pub backups_may_be_created: bool,
    pub note: String,
    pub display_command: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptResult {
    pub ok: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub backups: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoImportPlan {
    pub source: String,
    pub destination: String,
    pub source_type: String,
    pub display_command: String,
    pub affected_paths: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoImportResult {
    pub ok: bool,
    pub destination: String,
    pub repo_path: Option<String>,
    pub stdout: String,
    pub stderr: String,
}
