//! Saved cluster profiles and the `clusters.json` file that stores them.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cluster {
    pub id: Uuid,
    /// Display name shown in the sidebar.
    pub name: String,
    /// ssh target, verbatim: an alias from ~/.ssh/config or a hostname.
    pub host: String,
    /// Remote username. May be empty, in which case the alias's User applies.
    pub user: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Selection {
    All,
    Cluster { id: Uuid },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ClustersFile {
    #[serde(default)]
    pub clusters: Vec<Cluster>,
    #[serde(default)]
    pub selected: Option<Selection>,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "{e}"),
            ConfigError::Parse(e) => write!(f, "invalid JSON: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError::Parse(e)
    }
}

impl ClustersFile {
    /// A missing file is an empty config. Any other read failure, or
    /// malformed JSON, is an error so the caller can report it and avoid
    /// overwriting the file.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Ok(serde_json::from_str(&contents)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Inserts, or replaces the cluster with the same id.
    pub fn upsert(&mut self, cluster: Cluster) {
        match self.clusters.iter_mut().find(|c| c.id == cluster.id) {
            Some(existing) => *existing = cluster,
            None => self.clusters.push(cluster),
        }
    }

    /// Removes by id. Clears the selection if it pointed at that cluster.
    pub fn remove(&mut self, id: Uuid) {
        self.clusters.retain(|c| c.id != id);
        if self.selected == Some(Selection::Cluster { id }) {
            self.selected = None;
        }
    }

    pub fn find(&self, id: Uuid) -> Option<&Cluster> {
        self.clusters.iter().find(|c| c.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cluster(name: &str) -> Cluster {
        Cluster {
            id: Uuid::new_v4(),
            name: name.into(),
            host: format!("{name}.example.org"),
            user: "alice".into(),
        }
    }

    #[test]
    fn missing_file_loads_as_default() {
        let dir = tempfile::tempdir().unwrap();
        let file = ClustersFile::load(&dir.path().join("clusters.json")).unwrap();
        assert_eq!(file, ClustersFile::default());
    }

    #[test]
    fn save_then_load_round_trips_and_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("clusters.json");
        let c = cluster("snellius");
        let mut file = ClustersFile::default();
        file.upsert(c.clone());
        file.selected = Some(Selection::Cluster { id: c.id });
        file.save(&path).unwrap();

        let loaded = ClustersFile::load(&path).unwrap();
        assert_eq!(loaded, file);
    }

    #[test]
    fn malformed_file_is_a_parse_error_and_is_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clusters.json");
        std::fs::write(&path, "{ not json").unwrap();
        let err = ClustersFile::load(&path).unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");
    }

    #[test]
    fn selection_serialises_with_kind_tag() {
        let id = Uuid::nil();
        assert_eq!(
            serde_json::to_string(&Selection::All).unwrap(),
            r#"{"kind":"all"}"#
        );
        assert_eq!(
            serde_json::to_string(&Selection::Cluster { id }).unwrap(),
            r#"{"kind":"cluster","id":"00000000-0000-0000-0000-000000000000"}"#
        );
    }

    #[test]
    fn file_with_only_clusters_key_loads_with_no_selection() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clusters.json");
        std::fs::write(&path, r#"{"clusters":[]}"#).unwrap();
        let file = ClustersFile::load(&path).unwrap();
        assert_eq!(file.selected, None);
    }

    #[test]
    fn upsert_replaces_existing_by_id() {
        let mut file = ClustersFile::default();
        let mut c = cluster("a");
        file.upsert(c.clone());
        c.name = "renamed".into();
        file.upsert(c.clone());
        assert_eq!(file.clusters.len(), 1);
        assert_eq!(file.clusters[0].name, "renamed");
    }

    #[test]
    fn remove_clears_selection_if_it_pointed_at_the_removed_cluster() {
        let mut file = ClustersFile::default();
        let a = cluster("a");
        let b = cluster("b");
        file.upsert(a.clone());
        file.upsert(b.clone());
        file.selected = Some(Selection::Cluster { id: a.id });
        file.remove(a.id);
        assert_eq!(file.clusters, vec![b.clone()]);
        assert_eq!(file.selected, None);

        file.selected = Some(Selection::All);
        file.remove(b.id);
        assert!(file.clusters.is_empty());
        assert_eq!(file.selected, Some(Selection::All));
    }

    #[test]
    fn find_by_id() {
        let mut file = ClustersFile::default();
        let a = cluster("a");
        file.upsert(a.clone());
        assert_eq!(file.find(a.id), Some(&a));
        assert_eq!(file.find(Uuid::new_v4()), None);
    }
}
