use serde::{Deserialize, Serialize};
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
    pub designs: String,
    pub registry: String,
    pub scripts: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub repo: Option<AgentsRepo>,
    pub repo_error: Option<String>,
    pub generated_at: String,
    pub source_config: SourceConfigStatus,
    pub registry: McpRegistry,
    pub tools: Vec<ToolStatus>,
    pub skills: Vec<SkillItem>,
    pub commands: Vec<CommandItem>,
    pub designs: Vec<DesignItem>,
    pub mcp_statuses: BTreeMap<String, Vec<McpInstallStatus>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceConfigStatus {
    pub path: String,
    pub exists: bool,
    pub valid: bool,
    pub error: Option<String>,
    pub sources: Vec<ResourceSourceStatus>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSourceStatus {
    pub name: String,
    pub path: String,
    pub resolved_path: String,
    pub enabled: bool,
    pub resources: Vec<String>,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSourcesFile {
    #[serde(default)]
    pub sources: Vec<ResourceSourceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSourceConfig {
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default, rename = "ref")]
    pub git_ref: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub refresh: Option<bool>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub skills: Option<bool>,
    #[serde(default)]
    pub commands: Option<bool>,
    #[serde(default)]
    pub designs: Option<bool>,
    #[serde(default)]
    pub skills_path: Option<String>,
    #[serde(default)]
    pub commands_path: Option<String>,
    #[serde(default)]
    pub designs_path: Option<String>,
    #[serde(default)]
    pub skill_paths: Vec<String>,
    #[serde(default)]
    pub command_paths: Vec<String>,
    #[serde(default)]
    pub design_paths: Vec<String>,
    #[serde(default)]
    pub include_skills: Vec<String>,
    #[serde(default)]
    pub exclude_skills: Vec<String>,
    #[serde(default)]
    pub include_commands: Vec<String>,
    #[serde(default)]
    pub exclude_commands: Vec<String>,
    #[serde(default)]
    pub include_designs: Vec<String>,
    #[serde(default)]
    pub exclude_designs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillItem {
    pub name: String,
    pub description: String,
    pub path: String,
    pub file: String,
    pub source_name: String,
    pub source_path: String,
    pub source_kind: String,
    pub frontmatter: BTreeMap<String, String>,
    pub preview: String,
    pub installs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandItem {
    pub name: String,
    pub description: String,
    pub argument_hint: String,
    pub path: String,
    pub source_name: String,
    pub source_path: String,
    pub source_kind: String,
    pub frontmatter: BTreeMap<String, String>,
    pub preview: String,
    pub installs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignItem {
    pub name: String,
    pub description: String,
    pub path: String,
    pub file: String,
    pub source_name: String,
    pub source_path: String,
    pub source_kind: String,
    pub frontmatter: BTreeMap<String, String>,
    pub preview: String,
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
    pub has_headers: bool,
    pub has_environment: bool,
    pub enabled: bool,
    pub targets: BTreeMap<String, bool>,
    pub raw_json: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_command: Option<String>,
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
    pub branch: Option<String>,
    pub shallow: bool,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectiveSyncPlan {
    pub title: String,
    pub supported: bool,
    pub plans: Vec<ScriptPlan>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffPreview {
    pub action: String,
    pub title: String,
    pub sections: Vec<DiffSection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSection {
    pub title: String,
    pub path: String,
    pub section_type: String,
    pub status: String,
    pub diff: String,
}

// --- Feature 1: Runtime/About Panel ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub app_version: String,
    pub repo_root: String,
    pub repo_mode: String,
    pub bundled_script_count: usize,
    pub scripts: Vec<BundledScriptInfo>,
    pub repo_has_local_scripts: bool,
    pub platform: String,
    pub platform_arch: String,
    pub script_family: String,
    pub dependencies: BTreeMap<String, String>,
    pub agents_md_exists: bool,
    pub agents_md_size: u64,
    pub agents_md_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundledScriptInfo {
    pub name: String,
    pub checksum: String,
    pub size: usize,
}

// --- Feature 2: MCP Auth Status ---

// --- already added auth_status and auth_command to McpInstallStatus above ---

// --- Feature 4: Script Version Workflow ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptVersionInfo {
    pub name: String,
    pub bundled_checksum: String,
    pub bundled_size: usize,
    pub repo_path: Option<String>,
    pub repo_checksum: Option<String>,
    pub repo_size: Option<u64>,
    pub status: ScriptVersionStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ScriptVersionStatus {
    BundledMatchesRepo,
    BundledDiffersFromRepo,
    RepoMissing,
    NoRepoScript,
}

// --- Feature 6: Structured Script Output ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredOutput {
    pub summary: Vec<String>,
    pub changed: Vec<String>,
    pub skipped: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub backups: Vec<String>,
    pub auth_hints: Vec<String>,
    pub raw_stdout: String,
    pub raw_stderr: String,
    pub exit_code: Option<i32>,
}

// --- Feature 15: Repo Validation ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoValidation {
    pub path: String,
    pub issues: Vec<ValidationIssue>,
    pub severity_summary: SeveritySummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub code: String,
    pub severity: String,
    pub path: String,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeveritySummary {
    pub info: usize,
    pub warning: usize,
    pub error: usize,
}

// --- Feature 16: AGENTS.md Preview ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentsMdInfo {
    pub path: String,
    pub exists: bool,
    pub size: u64,
    pub last_modified: Option<String>,
    pub content: String,
    pub valid: bool,
    pub issues: Vec<String>,
}

// --- Feature 21: Local Logs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub action: String,
    pub repo_path: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
    pub backups: Vec<String>,
}

// --- Feature 10: MCP Registry Editor ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerFormData {
    pub name: String,
    pub description: String,
    pub server_type: String,
    pub transport: String,
    pub url: String,
    pub command: String,
    pub args: Vec<String>,
    pub headers: Option<BTreeMap<String, String>>,
    pub environment: Option<BTreeMap<String, String>>,
    pub enabled: bool,
    pub targets: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRegistryEditResult {
    pub ok: bool,
    pub message: String,
    pub validation_errors: Vec<String>,
    pub diff: String,
}

// --- Feature 11/12: Creation Wizards ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResult {
    pub ok: bool,
    pub path: String,
    pub message: String,
}

// --- Feature 13: Profiles ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub description: String,
    pub tools_enabled: Vec<String>,
    pub sync_globals: bool,
    pub sync_skills: bool,
    pub sync_commands: bool,
    pub sync_mcp: bool,
    pub selected_mcp_servers: Vec<String>,
    pub selected_skills: Vec<String>,
    pub selected_commands: Vec<String>,
}

// --- Feature 19: Packaging ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageArtifact {
    pub name: String,
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageResult {
    pub ok: bool,
    pub message: String,
    pub artifacts: Vec<PackageArtifact>,
    pub stdout: String,
    pub stderr: String,
}

// --- Feature 14: Selective Sync ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct SelectiveSyncConfig {
    pub tools: Vec<String>,
    pub categories: Vec<String>,
    pub selected_mcp_servers: Vec<String>,
}

// --- Feature 20: Update Check ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_url: Option<String>,
    pub up_to_date: bool,
    pub note: String,
}
