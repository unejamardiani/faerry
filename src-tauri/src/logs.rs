use crate::models::LogEntry;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LOGS_FILENAME: &str = "agent-manager-logs.json";

fn logs_path() -> PathBuf {
    let dir = std::env::temp_dir().join("faerry");
    dir.join(LOGS_FILENAME)
}

fn read_logs_raw() -> Vec<LogEntry> {
    let path = logs_path();
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(entries) => entries,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

pub fn append_log(entry: LogEntry) {
    let mut entries = read_logs_raw();
    entries.push(entry);
    // Keep only last 500 entries
    while entries.len() > 500 {
        entries.remove(0);
    }
    let path = logs_path();
    let _ = fs::create_dir_all(path.parent().unwrap_or(Path::new("")));
    let _ = fs::write(
        &path,
        serde_json::to_string_pretty(&entries).unwrap_or_default(),
    );
}

pub fn list_logs() -> Vec<LogEntry> {
    read_logs_raw()
}

pub fn clear_logs() {
    let path = logs_path();
    let _ = fs::remove_file(&path);
}

pub fn log_from_script_result(
    action: &str,
    repo_path: &str,
    command: &str,
    ok: bool,
    exit_code: Option<i32>,
    stdout: &str,
    stderr: &str,
    backups: &[String],
) -> LogEntry {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    LogEntry {
        timestamp: seconds.to_string(),
        action: action.into(),
        repo_path: repo_path.into(),
        command: command.into(),
        exit_code,
        ok,
        stdout: truncate(stdout, 10_000),
        stderr: truncate(stderr, 10_000),
        backups: backups.to_vec(),
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!("{}... (truncated, {} chars)", &text[..max], text.len())
    }
}
