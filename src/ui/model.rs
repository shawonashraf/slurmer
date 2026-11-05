//! Pure view logic: what the window shows for a given set of clusters,
//! selection and fetch results. No gpui types, so it is unit-tested.

use crate::config::{Cluster, Selection};
use crate::fetch::FetchError;
use crate::slurm::Job;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Badge colour for a Slurm job state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Green,
    Amber,
    Red,
    Blue,
    Grey,
}

/// Maps a Slurm job state (as printed by squeue %T) to a badge colour.
pub fn tone_for(state: &str) -> Tone {
    match state.trim().to_ascii_uppercase().as_str() {
        "RUNNING" | "COMPLETING" => Tone::Green,
        "PENDING" | "CONFIGURING" | "SUSPENDED" => Tone::Amber,
        "FAILED" | "CANCELLED" | "TIMEOUT" | "NODE_FAIL" | "OUT_OF_MEMORY" => Tone::Red,
        "COMPLETED" => Tone::Blue,
        _ => Tone::Grey,
    }
}

/// The selection to show at startup: the saved one if it still exists,
/// otherwise the first cluster, otherwise All.
pub fn initial_selection(saved: Option<&Selection>, clusters: &[Cluster]) -> Selection {
    match saved {
        Some(Selection::All) => Selection::All,
        Some(Selection::Cluster { id }) if clusters.iter().any(|c| c.id == *id) => {
            Selection::Cluster { id: *id }
        }
        _ => first_or_all(clusters),
    }
}

/// The selection after `deleted` was removed, or `None` if the current
/// selection is unaffected.
pub fn after_delete(sel: &Selection, deleted: Uuid, remaining: &[Cluster]) -> Option<Selection> {
    match sel {
        Selection::Cluster { id } if *id == deleted => Some(first_or_all(remaining)),
        _ => None,
    }
}

fn first_or_all(clusters: &[Cluster]) -> Selection {
    clusters
        .first()
        .map_or(Selection::All, |c| Selection::Cluster { id: c.id })
}

/// Sidebar status dot for one cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DotStatus {
    Loading,
    Error,
    Ok,
    Idle,
}

/// Per-cluster fetch results, keyed by cluster id, so switching clusters
/// shows the last fetch without a new ssh call.
#[derive(Debug, Default)]
pub struct Fetches {
    /// Last successful fetch.
    pub jobs: HashMap<Uuid, Vec<Job>>,
    /// Last fetch error; absent after a successful fetch.
    pub errors: HashMap<Uuid, String>,
    pub loading: HashSet<Uuid>,
    /// Refreshed at least once (success or error).
    pub fetched: HashSet<Uuid>,
}

impl Fetches {
    pub fn start(&mut self, id: Uuid) {
        self.loading.insert(id);
    }

    /// Records a finished fetch. A result for a cluster that is no longer
    /// in `known` (deleted while ssh ran) is dropped.
    pub fn finish(&mut self, id: Uuid, result: Result<Vec<Job>, FetchError>, known: &[Cluster]) {
        self.loading.remove(&id);
        if !known.iter().any(|c| c.id == id) {
            return;
        }
        match result {
            Ok(jobs) => {
                self.jobs.insert(id, jobs);
                self.errors.remove(&id);
            }
            Err(e) => {
                self.errors.insert(id, e.message);
            }
        }
        self.fetched.insert(id);
    }

    pub fn forget(&mut self, id: Uuid) {
        self.jobs.remove(&id);
        self.errors.remove(&id);
        self.loading.remove(&id);
        self.fetched.remove(&id);
    }

    pub fn is_loading(&self, id: Uuid) -> bool {
        self.loading.contains(&id)
    }

    pub fn any_loading(&self) -> bool {
        !self.loading.is_empty()
    }

    pub fn status(&self, id: Uuid) -> DotStatus {
        if self.loading.contains(&id) {
            DotStatus::Loading
        } else if self.errors.contains_key(&id) {
            DotStatus::Error
        } else if self.fetched.contains(&id) {
            DotStatus::Ok
        } else {
            DotStatus::Idle
        }
    }
}

/// One cluster's slice of the table.
#[derive(Debug, PartialEq)]
pub struct Group<'a> {
    pub cluster: &'a Cluster,
    pub jobs: &'a [Job],
    /// Set only in the All view; shown inline in place of rows. The single
    /// view shows its error as a strip above the table instead.
    pub error: Option<&'a str>,
    pub fetched: bool,
}

pub fn selected_cluster<'a>(sel: &Selection, clusters: &'a [Cluster]) -> Option<&'a Cluster> {
    match sel {
        Selection::All => None,
        Selection::Cluster { id } => clusters.iter().find(|c| c.id == *id),
    }
}

pub fn groups<'a>(sel: &Selection, clusters: &'a [Cluster], f: &'a Fetches) -> Vec<Group<'a>> {
    let is_all = *sel == Selection::All;
    let list: Vec<&Cluster> = if is_all {
        clusters.iter().collect()
    } else {
        selected_cluster(sel, clusters).into_iter().collect()
    };
    list.into_iter()
        .map(|cluster| Group {
            cluster,
            jobs: f.jobs.get(&cluster.id).map_or(&[][..], Vec::as_slice),
            error: if is_all {
                f.errors.get(&cluster.id).map(String::as_str)
            } else {
                None
            },
            fetched: f.fetched.contains(&cluster.id),
        })
        .collect()
}

/// What the main panel shows below the toolbar and strips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Body {
    NoClusters,
    NotRefreshed,
    /// Single view, error and no rows: the error strip says it all.
    ErrorOnly,
    /// Single view, fetched, no jobs.
    Empty,
    Table,
}

pub fn body(sel: &Selection, clusters: &[Cluster], f: &Fetches) -> Body {
    if clusters.is_empty() {
        return Body::NoClusters;
    }
    if !groups(sel, clusters, f).iter().any(|g| g.fetched) {
        return Body::NotRefreshed;
    }
    if let Some(c) = selected_cluster(sel, clusters) {
        let no_rows = f.jobs.get(&c.id).is_none_or(Vec::is_empty);
        if no_rows {
            return if f.errors.contains_key(&c.id) {
                Body::ErrorOnly
            } else {
                Body::Empty
            };
        }
    }
    Body::Table
}

pub fn title(sel: &Selection, clusters: &[Cluster]) -> String {
    match sel {
        Selection::All => "All clusters".to_string(),
        _ => selected_cluster(sel, clusters).map_or_else(String::new, |c| c.name.clone()),
    }
}

/// "N clusters" in the All view, `user@host` or `host` for one cluster.
pub fn subtitle(sel: &Selection, clusters: &[Cluster]) -> String {
    match sel {
        Selection::All => match clusters.len() {
            0 => String::new(),
            1 => "1 cluster".to_string(),
            n => format!("{n} clusters"),
        },
        _ => selected_cluster(sel, clusters).map_or_else(String::new, |c| {
            if c.user.is_empty() {
                c.host.clone()
            } else {
                format!("{}@{}", c.user, c.host)
            }
        }),
    }
}

/// Which window sides touch the screen edge (tiled or maximised). Mirrors
/// gpui's `Tiling` so this module stays free of gpui types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tiled {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

/// A window edge or corner the user can drag to resize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Resize handles to draw when slurmer draws its own window frame (GNOME on
/// Wayland). A tiled side has no handle, and a corner needs both of its
/// sides free.
pub fn resize_edges(tiled: Tiled) -> Vec<Edge> {
    let (t, b, l, r) = (!tiled.top, !tiled.bottom, !tiled.left, !tiled.right);
    [
        (t, Edge::Top),
        (b, Edge::Bottom),
        (l, Edge::Left),
        (r, Edge::Right),
        (t && l, Edge::TopLeft),
        (t && r, Edge::TopRight),
        (b && l, Edge::BottomLeft),
        (b && r, Edge::BottomRight),
    ]
    .into_iter()
    .filter_map(|(free, edge)| free.then_some(edge))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cluster(name: &str) -> Cluster {
        Cluster {
            id: Uuid::new_v4(),
            name: name.into(),
            host: format!("{name}-host"),
            user: String::new(),
        }
    }

    fn job(id: &str) -> Job {
        Job {
            id: id.into(),
            partition: "p".into(),
            name: "n".into(),
            state: "RUNNING".into(),
            time: "1:00".into(),
            nodes: "1".into(),
            reason: "node1".into(),
            time_limit: "2:00".into(),
            cpus: "4".into(),
            min_memory: "8G".into(),
        }
    }

    fn err(msg: &str) -> Result<Vec<Job>, FetchError> {
        Err(FetchError {
            message: msg.into(),
        })
    }

    fn one(c: &Cluster) -> Selection {
        Selection::Cluster { id: c.id }
    }

    #[test]
    fn tone_for_maps_states_case_and_space_insensitively() {
        assert_eq!(tone_for(" running "), Tone::Green);
        assert_eq!(tone_for("COMPLETING"), Tone::Green);
        assert_eq!(tone_for("PENDING"), Tone::Amber);
        assert_eq!(tone_for("SUSPENDED"), Tone::Amber);
        assert_eq!(tone_for("OUT_OF_MEMORY"), Tone::Red);
        assert_eq!(tone_for("TIMEOUT"), Tone::Red);
        assert_eq!(tone_for("COMPLETED"), Tone::Blue);
        assert_eq!(tone_for("WEIRD"), Tone::Grey);
    }

    #[test]
    fn initial_selection_prefers_saved_then_first_then_all() {
        let a = cluster("a");
        let b = cluster("b");
        let list = vec![a.clone(), b.clone()];
        assert_eq!(initial_selection(Some(&one(&b)), &list), one(&b));
        assert_eq!(
            initial_selection(Some(&Selection::All), &list),
            Selection::All
        );
        let gone = Selection::Cluster { id: Uuid::new_v4() };
        assert_eq!(initial_selection(Some(&gone), &list), one(&a));
        assert_eq!(initial_selection(None, &list), one(&a));
        assert_eq!(initial_selection(Some(&gone), &[]), Selection::All);
        assert_eq!(initial_selection(None, &[]), Selection::All);
    }

    #[test]
    fn after_delete_moves_off_the_deleted_cluster_only() {
        let a = cluster("a");
        let b = cluster("b");
        assert_eq!(
            after_delete(&one(&a), a.id, std::slice::from_ref(&b)),
            Some(one(&b))
        );
        assert_eq!(after_delete(&one(&a), a.id, &[]), Some(Selection::All));
        assert_eq!(after_delete(&one(&b), a.id, std::slice::from_ref(&b)), None);
        assert_eq!(after_delete(&Selection::All, a.id, &[b]), None);
    }

    #[test]
    fn finish_records_success_and_error() {
        let a = cluster("a");
        let known = vec![a.clone()];
        let mut f = Fetches::default();

        f.start(a.id);
        assert!(f.is_loading(a.id));
        assert!(f.any_loading());
        assert_eq!(f.status(a.id), DotStatus::Loading);

        f.finish(a.id, Ok(vec![job("1")]), &known);
        assert!(!f.any_loading());
        assert_eq!(f.jobs[&a.id], vec![job("1")]);
        assert_eq!(f.status(a.id), DotStatus::Ok);

        // An error keeps the previous rows.
        f.start(a.id);
        f.finish(a.id, err("boom"), &known);
        assert_eq!(f.errors[&a.id], "boom");
        assert_eq!(f.jobs[&a.id], vec![job("1")]);
        assert_eq!(f.status(a.id), DotStatus::Error);

        // A later success clears the error.
        f.start(a.id);
        f.finish(a.id, Ok(vec![]), &known);
        assert!(!f.errors.contains_key(&a.id));
        assert!(f.jobs[&a.id].is_empty());
    }

    #[test]
    fn finish_for_a_deleted_cluster_is_dropped() {
        let a = cluster("a");
        let mut f = Fetches::default();
        f.start(a.id);
        f.finish(a.id, Ok(vec![job("1")]), &[]);
        assert!(f.jobs.is_empty());
        assert!(f.errors.is_empty());
        assert!(f.fetched.is_empty());
        assert!(!f.any_loading());
    }

    #[test]
    fn forget_clears_everything_for_a_cluster() {
        let a = cluster("a");
        let mut f = Fetches::default();
        f.start(a.id);
        f.finish(a.id, err("x"), std::slice::from_ref(&a));
        f.forget(a.id);
        assert_eq!(f.status(a.id), DotStatus::Idle);
        assert!(f.errors.is_empty() && f.fetched.is_empty());
    }

    #[test]
    fn groups_carry_errors_only_in_the_all_view() {
        let a = cluster("a");
        let b = cluster("b");
        let list = vec![a.clone(), b.clone()];
        let mut f = Fetches::default();
        f.finish(a.id, err("boom"), &list);
        f.finish(b.id, Ok(vec![job("7")]), &list);

        let all = groups(&Selection::All, &list, &f);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].error, Some("boom"));
        assert_eq!(all[1].jobs, &[job("7")][..]);
        assert!(all[1].fetched);

        let single = groups(&one(&a), &list, &f);
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].error, None);
        assert!(single[0].fetched);
    }

    #[test]
    fn body_follows_the_main_panel_order() {
        let a = cluster("a");
        let b = cluster("b");
        let list = vec![a.clone(), b.clone()];
        let mut f = Fetches::default();

        assert_eq!(body(&Selection::All, &[], &f), Body::NoClusters);
        assert_eq!(body(&Selection::All, &list, &f), Body::NotRefreshed);
        assert_eq!(body(&one(&a), &list, &f), Body::NotRefreshed);

        f.finish(a.id, err("boom"), &list);
        assert_eq!(body(&one(&a), &list, &f), Body::ErrorOnly);
        // In All, the error shows inline in the table.
        assert_eq!(body(&Selection::All, &list, &f), Body::Table);

        f.finish(b.id, Ok(vec![]), &list);
        assert_eq!(body(&one(&b), &list, &f), Body::Empty);

        f.finish(b.id, Ok(vec![job("1")]), &list);
        assert_eq!(body(&one(&b), &list, &f), Body::Table);

        // Error after a successful fetch keeps showing the old rows.
        f.finish(b.id, err("later"), &list);
        assert_eq!(body(&one(&b), &list, &f), Body::Table);
    }

    #[test]
    fn titles_and_subtitles() {
        let mut a = cluster("a");
        let list1 = vec![a.clone()];
        assert_eq!(title(&Selection::All, &list1), "All clusters");
        assert_eq!(title(&one(&a), &list1), "a");
        assert_eq!(subtitle(&Selection::All, &[]), "");
        assert_eq!(subtitle(&Selection::All, &list1), "1 cluster");
        assert_eq!(
            subtitle(&Selection::All, &[a.clone(), cluster("b")]),
            "2 clusters"
        );
        assert_eq!(subtitle(&one(&a), &list1), "a-host");
        a.user = "snellius_user".into();
        assert_eq!(subtitle(&one(&a), &[a.clone()]), "snellius_user@a-host");
        let gone = Selection::Cluster { id: Uuid::new_v4() };
        assert_eq!(title(&gone, &list1), "");
        assert_eq!(subtitle(&gone, &list1), "");
    }

    #[test]
    fn floating_window_gets_all_eight_resize_handles() {
        let free = Tiled::default();
        let edges = resize_edges(free);
        assert_eq!(edges.len(), 8);
        for e in [
            Edge::Top,
            Edge::Bottom,
            Edge::Left,
            Edge::Right,
            Edge::TopLeft,
            Edge::TopRight,
            Edge::BottomLeft,
            Edge::BottomRight,
        ] {
            assert!(edges.contains(&e), "missing {e:?}");
        }
    }

    #[test]
    fn maximised_window_gets_no_resize_handles() {
        let all = Tiled {
            top: true,
            bottom: true,
            left: true,
            right: true,
        };
        assert!(resize_edges(all).is_empty());
    }

    #[test]
    fn left_snapped_window_resizes_only_from_its_free_right_side() {
        // Snapped to the left half: top, bottom and left touch the screen.
        let snapped = Tiled {
            top: true,
            bottom: true,
            left: true,
            right: false,
        };
        assert_eq!(resize_edges(snapped), vec![Edge::Right]);
    }

    #[test]
    fn corner_needs_both_of_its_sides_free() {
        let top_only = Tiled {
            top: true,
            ..Tiled::default()
        };
        let edges = resize_edges(top_only);
        assert!(!edges.contains(&Edge::Top));
        assert!(!edges.contains(&Edge::TopLeft));
        assert!(!edges.contains(&Edge::TopRight));
        assert!(edges.contains(&Edge::BottomLeft));
        assert!(edges.contains(&Edge::BottomRight));
    }
}
