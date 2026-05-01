// Profiles module - currently provides default profiles
// In v2, this will drive selective sync behavior
pub use crate::models::Profile;

/// Get default profiles if no custom profiles exist.
pub fn default_profiles() -> Vec<Profile> {
    vec![
        Profile {
            name: "Work".into(),
            description: "Full sync: all tools, all categories.".into(),
            tools_enabled: vec!["claude-code".into(), "codex".into(), "opencode".into(), "github-copilot-cli".into()],
            sync_globals: true,
            sync_skills: true,
            sync_commands: true,
            sync_mcp: true,
            selected_mcp_servers: vec![],
            selected_skills: vec![],
            selected_commands: vec![],
        },
        Profile {
            name: "Personal".into(),
            description: "Minimal sync: only Claude Code.".into(),
            tools_enabled: vec!["claude-code".into()],
            sync_globals: true,
            sync_skills: true,
            sync_commands: true,
            sync_mcp: true,
            selected_mcp_servers: vec![],
            selected_skills: vec![],
            selected_commands: vec![],
        },
        Profile {
            name: "Minimal".into(),
            description: "No sync: read-only mode.".into(),
            tools_enabled: vec![],
            sync_globals: false,
            sync_skills: false,
            sync_commands: false,
            sync_mcp: false,
            selected_mcp_servers: vec![],
            selected_skills: vec![],
            selected_commands: vec![],
        },
    ]
}
