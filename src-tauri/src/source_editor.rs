use crate::{
    models::{ResourceSourceConfig, ResourceSourcesFile, SourceConfigEditResult, SourceFormData},
    repo,
};
use std::{fs, path::Path};

pub fn validate_source(data: &SourceFormData) -> Vec<String> {
    let mut errors = Vec::new();
    let has_path = !data.path.trim().is_empty();
    let has_url = !data.url.trim().is_empty();

    if !has_path && !has_url {
        errors.push("Choose a local folder or enter a Git URL.".into());
    }
    if has_path && has_url {
        errors.push("Use either a local folder or a Git URL, not both.".into());
    }
    if has_url {
        let url = data.url.trim();
        let looks_like_git = url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("git@")
            || url.ends_with(".git");
        if !looks_like_git {
            errors.push("Git URL should be an https://, http://, git@, or .git URL.".into());
        }
    }
    if !data.skills && !data.commands && !data.designs {
        errors.push("Enable at least one content type.".into());
    }
    if !data.name.trim().is_empty()
        && !data
            .name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == ' ')
    {
        errors.push("Name can contain letters, numbers, spaces, hyphens, and underscores.".into());
    }

    errors
}

pub fn migrate_sources_json(repo_root: &str) -> SourceConfigEditResult {
    let root = Path::new(repo_root);
    let legacy = root.join(repo::LEGACY_SOURCES_FILENAME);
    let standard = repo::standard_source_config_path(root);
    if standard.exists() {
        return result(
            false,
            "faerry.json already exists.",
            Vec::new(),
            String::new(),
            &standard,
        );
    }
    if !legacy.exists() {
        let empty = ResourceSourcesFile {
            sources: Vec::new(),
        };
        return write_config(&standard, &empty, "Created faerry.json.");
    }
    match fs::copy(&legacy, &standard) {
        Ok(_) => result(
            true,
            "Migrated sources.json to faerry.json. The old file was left untouched.",
            Vec::new(),
            format!(
                "+ Created {}\n= Kept {}",
                repo::display_path(&standard),
                repo::display_path(&legacy)
            ),
            &standard,
        ),
        Err(error) => result(
            false,
            format!("Could not migrate sources.json: {error}"),
            Vec::new(),
            String::new(),
            &standard,
        ),
    }
}

pub fn edit_source_config(
    repo_root: &str,
    action: &str,
    index: Option<usize>,
    data: Option<&SourceFormData>,
) -> SourceConfigEditResult {
    let root = Path::new(repo_root);
    let path = repo::source_config_path(root);
    let write_path = if repo::is_legacy_source_config(&path) {
        repo::standard_source_config_path(root)
    } else {
        path
    };

    let mut file = match read_config(root) {
        Ok(file) => file,
        Err(error) => {
            return result(false, error, Vec::new(), String::new(), &write_path);
        }
    };

    let diff = match action {
        "add" => {
            let Some(data) = data else {
                return result(
                    false,
                    "Source data is required.",
                    Vec::new(),
                    String::new(),
                    &write_path,
                );
            };
            let validation = validate_source(data);
            if !validation.is_empty() {
                return result(
                    false,
                    "Source needs a few fixes.",
                    validation,
                    String::new(),
                    &write_path,
                );
            }
            let source = form_to_config(data);
            let label = source_label(&source);
            file.sources.push(source);
            format!("+ Added source: {label}")
        }
        "edit" => {
            let Some(index) = index else {
                return result(
                    false,
                    "Source index is required.",
                    Vec::new(),
                    String::new(),
                    &write_path,
                );
            };
            let Some(data) = data else {
                return result(
                    false,
                    "Source data is required.",
                    Vec::new(),
                    String::new(),
                    &write_path,
                );
            };
            if index >= file.sources.len() {
                return result(
                    false,
                    "Source not found.",
                    Vec::new(),
                    String::new(),
                    &write_path,
                );
            }
            let validation = validate_source(data);
            if !validation.is_empty() {
                return result(
                    false,
                    "Source needs a few fixes.",
                    validation,
                    String::new(),
                    &write_path,
                );
            }
            let old = source_label(&file.sources[index]);
            let source = form_to_config(data);
            let new = source_label(&source);
            file.sources[index] = source;
            format!("~ Updated source: {old} -> {new}")
        }
        "delete" => {
            let Some(index) = index else {
                return result(
                    false,
                    "Source index is required.",
                    Vec::new(),
                    String::new(),
                    &write_path,
                );
            };
            if index >= file.sources.len() {
                return result(
                    false,
                    "Source not found.",
                    Vec::new(),
                    String::new(),
                    &write_path,
                );
            }
            let removed = file.sources.remove(index);
            format!("- Removed source: {}", source_label(&removed))
        }
        "toggle" => {
            let Some(index) = index else {
                return result(
                    false,
                    "Source index is required.",
                    Vec::new(),
                    String::new(),
                    &write_path,
                );
            };
            if index >= file.sources.len() {
                return result(
                    false,
                    "Source not found.",
                    Vec::new(),
                    String::new(),
                    &write_path,
                );
            }
            let next = !file.sources[index].enabled.unwrap_or(true);
            file.sources[index].enabled = Some(next);
            format!(
                "~ {} source: {}",
                if next { "Enabled" } else { "Disabled" },
                source_label(&file.sources[index])
            )
        }
        _ => {
            return result(
                false,
                format!("Unknown source action: {action}"),
                Vec::new(),
                String::new(),
                &write_path,
            );
        }
    };

    write_config(&write_path, &file, "Source configuration saved.").with_diff(diff)
}

fn read_config(root: &Path) -> Result<ResourceSourcesFile, String> {
    let path = repo::source_config_path(root);
    if !path.exists() {
        return Ok(ResourceSourcesFile {
            sources: Vec::new(),
        });
    }
    let text = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn write_config(path: &Path, file: &ResourceSourcesFile, message: &str) -> SourceConfigEditResult {
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            return result(false, error.to_string(), Vec::new(), String::new(), path);
        }
    }
    if path.exists() {
        let backup = path.with_extension(format!(
            "{}.{}.bak",
            path.extension()
                .map(|ext| ext.to_string_lossy().to_string())
                .unwrap_or_else(|| "json".into()),
            timestamp()
        ));
        if let Ok(text) = fs::read_to_string(path) {
            let _ = fs::write(backup, text);
        }
    }
    let text = match serde_json::to_string_pretty(file) {
        Ok(text) => format!("{text}\n"),
        Err(error) => return result(false, error.to_string(), Vec::new(), String::new(), path),
    };
    match fs::write(path, text) {
        Ok(_) => result(true, message, Vec::new(), String::new(), path),
        Err(error) => result(false, error.to_string(), Vec::new(), String::new(), path),
    }
}

fn form_to_config(data: &SourceFormData) -> ResourceSourceConfig {
    ResourceSourceConfig {
        name: optional(data.name.trim()),
        path: optional(data.path.trim()),
        url: optional(data.url.trim()),
        git_ref: optional(data.git_ref.trim()),
        branch: None,
        refresh: Some(data.refresh),
        enabled: Some(data.enabled),
        skills: Some(data.skills),
        commands: Some(data.commands),
        designs: Some(data.designs),
        skills_path: optional(data.skills_path.trim()),
        commands_path: optional(data.commands_path.trim()),
        designs_path: optional(data.designs_path.trim()),
        skill_paths: clean_vec(&data.skill_paths),
        command_paths: clean_vec(&data.command_paths),
        design_paths: clean_vec(&data.design_paths),
        include_skills: clean_vec(&data.include_skills),
        exclude_skills: clean_vec(&data.exclude_skills),
        include_commands: clean_vec(&data.include_commands),
        exclude_commands: clean_vec(&data.exclude_commands),
        include_designs: clean_vec(&data.include_designs),
        exclude_designs: clean_vec(&data.exclude_designs),
    }
}

fn clean_vec(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn optional(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

fn source_label(source: &ResourceSourceConfig) -> String {
    source
        .name
        .clone()
        .or_else(|| source.url.clone())
        .or_else(|| source.path.clone())
        .unwrap_or_else(|| "source".into())
}

fn result(
    ok: bool,
    message: impl Into<String>,
    validation_errors: Vec<String>,
    diff: String,
    path: &Path,
) -> SourceConfigEditResult {
    SourceConfigEditResult {
        ok,
        message: message.into(),
        validation_errors,
        diff,
        path: repo::display_path(path),
    }
}

trait WithDiff {
    fn with_diff(self, diff: String) -> Self;
}

impl WithDiff for SourceConfigEditResult {
    fn with_diff(mut self, diff: String) -> Self {
        self.diff = diff;
        self
    }
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ResourceSourcesFile;
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn add_source_writes_standard_faerry_config() {
        let root = temp_path("add-source");
        fs::create_dir_all(&root).unwrap();
        let form = source_form("./shared", "");

        let result =
            edit_source_config(&repo::display_path(&root), "add".into(), None, Some(&form));

        assert!(result.ok, "{}", result.message);
        assert!(root.join(repo::FAERRY_CONFIG_FILENAME).exists());

        let file = read_sources(root.join(repo::FAERRY_CONFIG_FILENAME));
        assert_eq!(file.sources.len(), 1);
        assert_eq!(file.sources[0].name.as_deref(), Some("Shared workspace"));
        assert_eq!(file.sources[0].path.as_deref(), Some("./shared"));
        assert_eq!(file.sources[0].enabled, Some(true));
        assert_eq!(file.sources[0].skills, Some(true));
        assert_eq!(file.sources[0].commands, Some(false));
        assert_eq!(file.sources[0].designs, Some(true));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn editing_legacy_sources_writes_standard_faerry_config() {
        let root = temp_path("legacy-edit");
        fs::create_dir_all(&root).unwrap();
        let legacy = root.join(repo::LEGACY_SOURCES_FILENAME);
        fs::write(
            &legacy,
            r#"{
  "sources": [
    {
      "name": "Legacy",
      "path": "./old",
      "enabled": true,
      "skills": true
    }
  ]
}
"#,
        )
        .unwrap();
        let form = source_form("./modern", "");

        let result = edit_source_config(
            &repo::display_path(&root),
            "edit".into(),
            Some(0),
            Some(&form),
        );

        assert!(result.ok, "{}", result.message);
        assert!(root.join(repo::FAERRY_CONFIG_FILENAME).exists());
        assert!(legacy.exists());

        let file = read_sources(root.join(repo::FAERRY_CONFIG_FILENAME));
        assert_eq!(file.sources.len(), 1);
        assert_eq!(file.sources[0].name.as_deref(), Some("Shared workspace"));
        assert_eq!(file.sources[0].path.as_deref(), Some("./modern"));
        assert!(fs::read_to_string(legacy).unwrap().contains("Legacy"));

        let _ = fs::remove_dir_all(root);
    }

    fn source_form(path: &str, url: &str) -> SourceFormData {
        SourceFormData {
            name: "Shared workspace".into(),
            path: path.into(),
            url: url.into(),
            git_ref: String::new(),
            refresh: true,
            enabled: true,
            skills: true,
            commands: false,
            designs: true,
            skills_path: String::new(),
            commands_path: String::new(),
            designs_path: String::new(),
            skill_paths: vec![],
            command_paths: vec![],
            design_paths: vec![],
            include_skills: vec![],
            exclude_skills: vec![],
            include_commands: vec![],
            exclude_commands: vec![],
            include_designs: vec![],
            exclude_designs: vec![],
        }
    }

    fn read_sources(path: PathBuf) -> ResourceSourcesFile {
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    fn temp_path(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("faerry-source-editor-test-{label}-{suffix}"))
    }
}
