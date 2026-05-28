use crate::models::{McpRegistryEditResult, McpServerFormData};
use serde_json::{Map, Value};
use std::fs;

/// Validate MCP server form data.
pub fn validate_server(data: &McpServerFormData) -> Vec<String> {
    let mut errors = Vec::new();

    if data.name.is_empty() {
        errors.push("Server name is required.".into());
    }
    if !data
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        errors.push(
            "Server name must contain only alphanumeric characters, hyphens, and underscores."
                .into(),
        );
    }

    match data.server_type.as_str() {
        "remote" => {
            if data.url.is_empty() {
                errors.push("Remote servers require a URL.".into());
            }
            if !data.command.is_empty() {
                errors.push("Remote servers should not have a command.".into());
            }
        }
        "stdio" => {
            if data.command.is_empty() {
                errors.push("Stdio servers require a command.".into());
            }
            if !data.url.is_empty() {
                errors.push("Stdio servers should not have a URL.".into());
            }
        }
        _ => {
            errors.push(format!(
                "Unknown server type: {}. Use 'remote' or 'stdio'.",
                data.server_type
            ));
        }
    }

    if !data.transport.is_empty() && !["http", "sse", "stdio"].contains(&data.transport.as_str()) {
        errors.push(format!(
            "Unknown transport: {}. Use 'http', 'sse', or 'stdio'.",
            data.transport
        ));
    }

    errors
}

/// Apply an MCP server edit to the registry file.
pub fn edit_server(
    registry_path: &str,
    name: &str,
    action: &str, // "add", "edit", "delete"
    data: Option<&McpServerFormData>,
) -> McpRegistryEditResult {
    let text = match fs::read_to_string(registry_path) {
        Ok(t) => t,
        Err(e) => {
            return McpRegistryEditResult {
                ok: false,
                message: format!("Cannot read registry: {e}"),
                validation_errors: Vec::new(),
                diff: String::new(),
            };
        }
    };

    let mut parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            return McpRegistryEditResult {
                ok: false,
                message: format!("Invalid registry JSON: {e}"),
                validation_errors: Vec::new(),
                diff: String::new(),
            };
        }
    };

    if parsed.get("servers").and_then(Value::as_object).is_none() {
        if let Some(object) = parsed.as_object_mut() {
            object.insert("servers".into(), Value::Object(Map::new()));
        } else {
            return McpRegistryEditResult {
                ok: false,
                message: "Registry root must be a JSON object.".into(),
                validation_errors: vec![
                    "mcp/servers.json must contain a JSON object at the root.".into()
                ],
                diff: String::new(),
            };
        }
    }
    let servers = parsed
        .get_mut("servers")
        .and_then(Value::as_object_mut)
        .unwrap();

    let diff = match action {
        "add" => {
            if let Some(data) = data {
                if servers.contains_key(name) {
                    return McpRegistryEditResult {
                        ok: false,
                        message: format!("Server '{name}' already exists."),
                        validation_errors: vec![format!(
                            "Server '{name}' already exists in the registry."
                        )],
                        diff: String::new(),
                    };
                }
                let validation = validate_server(data);
                if !validation.is_empty() {
                    return McpRegistryEditResult {
                        ok: false,
                        message: "Form validation failed.".into(),
                        validation_errors: validation,
                        diff: String::new(),
                    };
                }
                let new_server = form_to_value(data);
                servers.insert(name.to_string(), new_server);
            }
            format!("+ Added server: {name}")
        }
        "edit" => {
            if let Some(data) = data {
                if !servers.contains_key(name) {
                    return McpRegistryEditResult {
                        ok: false,
                        message: format!("Server '{name}' not found."),
                        validation_errors: vec![format!(
                            "Server '{name}' does not exist in the registry."
                        )],
                        diff: String::new(),
                    };
                }
                let validation = validate_server(data);
                if !validation.is_empty() {
                    return McpRegistryEditResult {
                        ok: false,
                        message: "Form validation failed.".into(),
                        validation_errors: validation,
                        diff: String::new(),
                    };
                }
                let old = servers.get(name).cloned();
                servers.insert(name.to_string(), form_to_value(data));
                format!(
                    "- Old: {}\n+ New: {}",
                    serde_json::to_string_pretty(&old).unwrap_or_default(),
                    serde_json::to_string_pretty(servers.get(name).unwrap_or(&Value::Null))
                        .unwrap_or_default()
                )
            } else {
                format!("No changes for {name}")
            }
        }
        "delete" => {
            if !servers.contains_key(name) {
                return McpRegistryEditResult {
                    ok: false,
                    message: format!("Server '{name}' not found."),
                    validation_errors: vec![format!("Server '{name}' does not exist.")],
                    diff: String::new(),
                };
            }
            let _removed = servers.remove(name);
            format!("- Removed server: {name}")
        }
        _ => {
            return McpRegistryEditResult {
                ok: false,
                message: format!("Unknown action: {action}"),
                validation_errors: Vec::new(),
                diff: String::new(),
            };
        }
    };

    // Create backup
    let backup_path = format!("{registry_path}.{}.bak", timestamp());
    let _ = fs::write(&backup_path, &text);

    let new_text = serde_json::to_string_pretty(&parsed).unwrap_or_default();
    fs::write(registry_path, &new_text)
        .map_err(|e| e.to_string())
        .ok();

    McpRegistryEditResult {
        ok: true,
        message: format!("Registry updated. Backup saved to {backup_path}"),
        validation_errors: Vec::new(),
        diff,
    }
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn form_to_value(data: &McpServerFormData) -> Value {
    let mut map = Map::new();
    if !data.description.is_empty() {
        map.insert(
            "description".into(),
            Value::String(data.description.clone()),
        );
    }
    map.insert("type".into(), Value::String(data.server_type.clone()));
    map.insert("transport".into(), Value::String(data.transport.clone()));
    if !data.url.is_empty() {
        map.insert("url".into(), Value::String(data.url.clone()));
    }
    if !data.command.is_empty() {
        map.insert("command".into(), Value::String(data.command.clone()));
    }
    if !data.args.is_empty() {
        map.insert(
            "args".into(),
            Value::Array(data.args.iter().map(|a| Value::String(a.clone())).collect()),
        );
    }
    if let Some(headers) = &data.headers {
        map.insert(
            "headers".into(),
            Value::Object(
                headers
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect(),
            ),
        );
    }
    if let Some(env) = &data.environment {
        map.insert(
            "environment".into(),
            Value::Object(
                env.iter()
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect(),
            ),
        );
    }
    map.insert("enabled".into(), Value::Bool(data.enabled));
    if !data.targets.is_empty() {
        map.insert(
            "targets".into(),
            Value::Object(
                data.targets
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::Bool(*v)))
                    .collect(),
            ),
        );
    }
    Value::Object(map)
}
