//! Tauri commands: the only surface the frontend talks to.

use crate::config::{Cluster, ClustersFile, Selection};
use crate::fetch::{fetch_jobs as run_fetch, FetchError};
use crate::slurm::Job;
use crate::ssh_config::{default_config_path, load_ssh_hosts, SshHost};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;

/// Managed application state: the config file path, its in-memory
/// contents, and the error (if any) from loading it at startup.
///
/// `load_error` is a `Mutex` (not a plain field) because a successful
/// explicit save clears it at runtime: once the user saves a cluster,
/// the file on disk is well-formed again and no longer needs guarding.
pub struct AppState {
    pub path: PathBuf,
    pub file: Mutex<ClustersFile>,
    pub load_error: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(path: PathBuf) -> Self {
        let (file, load_error) = match ClustersFile::load(&path) {
            Ok(file) => (file, None),
            Err(e) => (
                ClustersFile::default(),
                Some(format!("Could not read {}: {e}", path.display())),
            ),
        };
        Self {
            path,
            file: Mutex::new(file),
            load_error: Mutex::new(load_error),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ClustersResponse {
    pub clusters: Vec<Cluster>,
    pub selected: Option<Selection>,
    pub load_error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClusterInput {
    pub id: Option<Uuid>,
    pub name: String,
    pub host: String,
    pub user: String,
}

/// Trims fields, requires name and host, keeps or mints the id.
pub fn validate_input(input: ClusterInput) -> Result<Cluster, String> {
    let name = input.name.trim().to_string();
    let host = input.host.trim().to_string();
    let user = input.user.trim().to_string();
    if name.is_empty() || host.is_empty() {
        return Err("Name and host are required".to_string());
    }
    Ok(Cluster {
        id: input.id.unwrap_or_else(Uuid::new_v4),
        name,
        host,
        user,
    })
}

/// Upserts into the in-memory file and writes it to disk. A successful
/// save means the file on disk is well-formed again, so it clears any
/// previously recorded load error.
pub fn apply_save(state: &AppState, cluster: Cluster) -> Result<(), String> {
    let mut file = state.file.lock().map_err(|e| e.to_string())?;
    file.upsert(cluster);
    file.save(&state.path).map_err(|e| e.to_string())?;
    let mut load_error = state.load_error.lock().map_err(|e| e.to_string())?;
    *load_error = None;
    Ok(())
}

/// Removes a cluster and saves, unless the config file failed to load at
/// startup — in that case the in-memory state is a throwaway default, so
/// deleting from it and saving would overwrite the still-malformed file
/// on disk with an empty one. Refuses instead, touching neither memory
/// nor disk.
pub fn apply_delete(state: &AppState, id: Uuid) -> Result<(), String> {
    let mut file = state.file.lock().map_err(|e| e.to_string())?;
    let had_load_error = state
        .load_error
        .lock()
        .map_err(|e| e.to_string())?
        .is_some();
    if had_load_error {
        return Err("clusters.json could not be read; add a cluster to reset it".to_string());
    }
    file.remove(id);
    file.save(&state.path).map_err(|e| e.to_string())
}

/// Updates the selection in memory always. Only persists it to disk when
/// the config file loaded cleanly; while a load error is outstanding,
/// selection is a best-effort UI convenience and must not overwrite the
/// still-malformed file.
pub fn apply_selection(state: &AppState, selection: Option<Selection>) -> Result<(), String> {
    let mut file = state.file.lock().map_err(|e| e.to_string())?;
    file.selected = selection;
    let had_load_error = state
        .load_error
        .lock()
        .map_err(|e| e.to_string())?
        .is_some();
    if had_load_error {
        return Ok(());
    }
    file.save(&state.path).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_clusters(state: State<'_, AppState>) -> Result<ClustersResponse, String> {
    let file = state.file.lock().map_err(|e| e.to_string())?;
    let load_error = state.load_error.lock().map_err(|e| e.to_string())?.clone();
    Ok(ClustersResponse {
        clusters: file.clusters.clone(),
        selected: file.selected.clone(),
        load_error,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_cluster(state: State<'_, AppState>, input: ClusterInput) -> Result<Cluster, String> {
    let cluster = validate_input(input)?;
    apply_save(&state, cluster.clone())?;
    Ok(cluster)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_cluster(state: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    apply_delete(&state, id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_selection(
    state: State<'_, AppState>,
    selection: Option<Selection>,
) -> Result<(), String> {
    apply_selection(&state, selection)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_ssh_hosts() -> Vec<SshHost> {
    default_config_path()
        .map(|path| load_ssh_hosts(&path))
        .unwrap_or_default()
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fetch_jobs(
    state: State<'_, AppState>,
    cluster_id: Uuid,
) -> Result<Vec<Job>, FetchError> {
    // Copy what we need out of the mutex before awaiting.
    let (host, user) = {
        let file = state.file.lock().map_err(|e| FetchError {
            message: e.to_string(),
        })?;
        let cluster = file.find(cluster_id).ok_or_else(|| FetchError {
            message: "Unknown cluster".to_string(),
        })?;
        (cluster.host.clone(), cluster.user.clone())
    };
    run_fetch(&host, &user).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_with_missing_file_has_no_load_error() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(dir.path().join("clusters.json"));
        assert_eq!(*state.load_error.lock().unwrap(), None);
        assert!(state.file.lock().unwrap().clusters.is_empty());
    }

    #[test]
    fn app_state_with_malformed_file_reports_and_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clusters.json");
        std::fs::write(&path, "nope").unwrap();
        let state = AppState::new(path.clone());
        let err = state
            .load_error
            .lock()
            .unwrap()
            .clone()
            .expect("load error");
        assert!(err.contains("clusters.json"), "{err}");
        assert!(state.file.lock().unwrap().clusters.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "nope");
    }

    #[test]
    fn validate_input_trims_and_requires_name_and_host() {
        let ok = validate_input(ClusterInput {
            id: None,
            name: "  Snellius ".into(),
            host: " Snellius-Large ".into(),
            user: " alice ".into(),
        })
        .unwrap();
        assert_eq!(ok.name, "Snellius");
        assert_eq!(ok.host, "Snellius-Large");
        assert_eq!(ok.user, "alice");

        let missing_name = validate_input(ClusterInput {
            id: None,
            name: "  ".into(),
            host: "h".into(),
            user: "".into(),
        });
        assert_eq!(missing_name.unwrap_err(), "Name and host are required");

        let missing_host = validate_input(ClusterInput {
            id: None,
            name: "n".into(),
            host: "".into(),
            user: "".into(),
        });
        assert!(missing_host.is_err());
    }

    #[test]
    fn validate_input_keeps_existing_id_or_mints_one() {
        let id = Uuid::new_v4();
        let kept = validate_input(ClusterInput {
            id: Some(id),
            name: "n".into(),
            host: "h".into(),
            user: "".into(),
        })
        .unwrap();
        assert_eq!(kept.id, id);

        let minted = validate_input(ClusterInput {
            id: None,
            name: "n".into(),
            host: "h".into(),
            user: "".into(),
        })
        .unwrap();
        assert_ne!(minted.id, Uuid::nil());
    }

    #[test]
    fn apply_save_persists_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(dir.path().join("clusters.json"));
        let cluster = validate_input(ClusterInput {
            id: None,
            name: "n".into(),
            host: "h".into(),
            user: "u".into(),
        })
        .unwrap();
        apply_save(&state, cluster.clone()).unwrap();
        let reloaded = ClustersFile::load(&state.path).unwrap();
        assert_eq!(reloaded.clusters, vec![cluster]);
    }

    #[test]
    fn apply_delete_on_malformed_file_is_refused_and_leaves_disk_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clusters.json");
        std::fs::write(&path, "nope").unwrap();
        let state = AppState::new(path.clone());

        let err = apply_delete(&state, Uuid::new_v4()).unwrap_err();

        assert!(err.contains("clusters.json"), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "nope");
    }

    #[test]
    fn apply_selection_on_malformed_file_updates_memory_but_not_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clusters.json");
        std::fs::write(&path, "nope").unwrap();
        let state = AppState::new(path.clone());

        apply_selection(&state, Some(Selection::All)).unwrap();

        assert_eq!(state.file.lock().unwrap().selected, Some(Selection::All));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "nope");
    }

    #[test]
    fn apply_save_on_malformed_file_recovers_and_clears_load_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clusters.json");
        std::fs::write(&path, "nope").unwrap();
        let state = AppState::new(path.clone());
        let cluster = validate_input(ClusterInput {
            id: None,
            name: "n".into(),
            host: "h".into(),
            user: "u".into(),
        })
        .unwrap();

        apply_save(&state, cluster.clone()).unwrap();

        let reloaded = ClustersFile::load(&state.path).unwrap();
        assert_eq!(reloaded.clusters, vec![cluster]);
        assert_eq!(*state.load_error.lock().unwrap(), None);
    }

    #[test]
    fn apply_delete_on_healthy_state_removes_cluster_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(dir.path().join("clusters.json"));
        let cluster = validate_input(ClusterInput {
            id: None,
            name: "n".into(),
            host: "h".into(),
            user: "u".into(),
        })
        .unwrap();
        apply_save(&state, cluster.clone()).unwrap();

        apply_delete(&state, cluster.id).unwrap();

        let reloaded = ClustersFile::load(&state.path).unwrap();
        assert!(reloaded.clusters.is_empty());
    }
}
