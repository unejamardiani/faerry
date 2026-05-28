use std::process::Command;

pub fn auth_command_for_server(tool: &str, server: &str, server_type: &str) -> Option<String> {
    match tool {
        "claude-code" => Some("Open Claude Code and run /mcp".into()),
        "codex" => {
            if server_type == "remote" {
                Some(format!("codex mcp login {server}"))
            } else {
                None
            }
        }
        "opencode" => {
            if server_type == "remote" {
                Some(format!("opencode mcp auth {server}"))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn check_auth_availability(tool: &str) -> (String, Option<String>) {
    let cli_available = match tool {
        "claude-code" => command_available("claude"),
        "codex" => command_available("codex"),
        "opencode" => command_available("opencode"),
        _ => false,
    };

    if !cli_available {
        let cmd = match tool {
            "claude-code" => "claude",
            "codex" => "codex",
            "opencode" => "opencode",
            _ => tool,
        };
        return (
            "cli-missing".into(),
            Some(format!("{cmd} CLI is not available on PATH.")),
        );
    }

    match tool {
        "codex" => check_codex_auth(),
        "opencode" => check_opencode_auth(),
        "claude-code" => check_claude_auth(),
        _ => ("unknown".into(), None),
    }
}

fn check_claude_auth() -> (String, Option<String>) {
    let output = Command::new("claude").args(["mcp", "list"]).output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            if text.contains("Not authenticated")
                || text.contains("requires login")
                || text.contains("credential")
            {
                (
                    "needs-auth".into(),
                    Some("MCP server requires authentication. Run /mcp in Claude Code.".into()),
                )
            } else {
                (
                    "authenticated".into(),
                    Some("Claude Code MCP list is available and shows the server.".into()),
                )
            }
        }
        _ => (
            "unknown".into(),
            Some("Could not determine auth status. Run /mcp in Claude Code to verify.".into()),
        ),
    }
}

fn check_codex_auth() -> (String, Option<String>) {
    let output = Command::new("codex")
        .args(["mcp", "auth", "status"])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            if text.contains("unauthorized") || text.contains("needs") || text.contains("login") {
                (
                    "needs-auth".into(),
                    Some("Codex reports the server needs authentication.".into()),
                )
            } else {
                (
                    "authenticated".into(),
                    Some("Codex reports authentication is configured.".into()),
                )
            }
        }
        _ => (
            "unknown".into(),
            Some("Codex auth status command not available. Check config manually.".into()),
        ),
    }
}

fn check_opencode_auth() -> (String, Option<String>) {
    let output = Command::new("opencode")
        .args(["mcp", "auth", "status"])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            if text.contains("unauthorized") || text.contains("needs") || text.contains("login") {
                (
                    "needs-auth".into(),
                    Some("OpenCode reports the server needs authentication.".into()),
                )
            } else {
                (
                    "authenticated".into(),
                    Some("OpenCode reports authentication is configured.".into()),
                )
            }
        }
        _ => (
            "unknown".into(),
            Some("OpenCode auth status command not available. Check config manually.".into()),
        ),
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
