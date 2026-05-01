mod bundled;
mod models;
mod preview;
mod repo;
mod scripts;
mod status;

use models::{AppState, DiffPreview, RepoImportPlan, RepoImportResult, ScriptPlan, ScriptResult};

#[tauri::command]
fn get_state(repo_path: Option<String>) -> Result<AppState, String> {
    Ok(status::build_state(repo_path))
}

#[tauri::command]
fn plan_action(action: String, repo_path: Option<String>) -> Result<ScriptPlan, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|error| error.to_string())?;
    scripts::plan_action(&repo, &action)
}

#[tauri::command]
async fn run_action(action: String, repo_path: Option<String>) -> Result<ScriptResult, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|error| error.to_string())?;
    let plan = scripts::plan_action(&repo, &action)?;
    scripts::run_plan(&plan)
}

#[tauri::command]
fn preview_action(action: String, repo_path: Option<String>) -> Result<DiffPreview, String> {
    let repo = repo::detect_repo_with_override(repo_path).map_err(|error| error.to_string())?;
    preview::preview_action(&repo, &action)
}

#[tauri::command]
fn choose_repo_path() -> Result<Option<String>, String> {
    scripts::choose_repo_path()
}

#[tauri::command]
fn plan_repo_import(source: String, destination: String) -> Result<RepoImportPlan, String> {
    scripts::plan_repo_import(&source, &destination)
}

#[tauri::command]
async fn run_repo_import(source: String, destination: String) -> Result<RepoImportResult, String> {
    scripts::run_repo_import(&source, &destination)
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    scripts::open_path(&path)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_state,
            plan_action,
            run_action,
            preview_action,
            choose_repo_path,
            plan_repo_import,
            run_repo_import,
            open_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agents Manager");
}
