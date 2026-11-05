//! The job table: one row per job, grouped by cluster in the All view.

use super::main_panel::error_block;
use super::model::Group;
use super::theme::theme;
use super::widgets::{badge, mono};
use crate::slurm::Job;
use gpui::{div, prelude::*, px, AnyElement, App, Div, FontWeight, SharedString, Stateful};

/// `None` width means the column takes the remaining space.
struct Column {
    label: &'static str,
    width: Option<f32>,
    right: bool,
}

const fn col(label: &'static str, width: Option<f32>, right: bool) -> Column {
    Column {
        label,
        width,
        right,
    }
}

const CLUSTER: Column = col("Cluster", Some(120.), false);
const COLUMNS: [Column; 10] = [
    col("Job ID", Some(100.), false),
    col("Partition", Some(110.), false),
    col("Name", None, false),
    col("State", Some(130.), false),
    col("Time", Some(90.), true),
    col("Nodes", Some(70.), true),
    col("Reason", Some(140.), false),
    col("Time Limit", Some(90.), true),
    col("CPUs", Some(60.), true),
    col("Min Memory", Some(100.), true),
];

const MIN_WIDTH: f32 = 900.;

fn cell(c: &Column) -> Div {
    let d = div()
        .flex()
        .items_center()
        .px(px(10.))
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis();
    let d = match c.width {
        Some(w) => d.w(px(w)).flex_none(),
        None => d.flex_1().min_w_0(),
    };
    if c.right {
        d.justify_end()
    } else {
        d
    }
}

pub fn render(groups: &[Group<'_>], show_cluster: bool, cx: &App) -> Stateful<Div> {
    let t = theme(cx).clone();

    let columns = || {
        show_cluster
            .then_some(&CLUSTER)
            .into_iter()
            .chain(COLUMNS.iter())
    };

    let header = div()
        .flex()
        .flex_none()
        .h(px(32.))
        .border_b_1()
        .border_color(t.border)
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(t.muted)
        .children(columns().map(|c| cell(c).child(c.label.to_uppercase())));

    let mut rows: Vec<AnyElement> = Vec::new();
    for g in groups {
        let note = |text: &str| -> AnyElement {
            let mut d = div()
                .flex()
                .gap_1()
                .px(px(10.))
                .py(px(7.))
                .border_b_1()
                .border_color(t.border)
                .text_color(t.muted);
            if show_cluster {
                d = d.child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(t.text)
                        .child(SharedString::from(format!("{}:", g.cluster.name))),
                );
            }
            d.child(SharedString::from(text.to_string()))
                .into_any_element()
        };

        if let Some(err) = g.error {
            rows.push(
                div()
                    .px(px(10.))
                    .py_1()
                    .border_b_1()
                    .border_color(t.border)
                    .child(error_block(
                        format!("group-error-{}", g.cluster.id),
                        show_cluster.then_some(g.cluster.name.as_str()),
                        err,
                        &g.cluster.host,
                        cx,
                    ))
                    .into_any_element(),
            );
        } else if !g.fetched {
            rows.push(note("not refreshed yet"));
        } else if g.jobs.is_empty() {
            rows.push(note("no jobs in the queue"));
        } else {
            for job in g.jobs {
                rows.push(job_row(
                    job,
                    show_cluster.then_some(g.cluster.name.as_str()),
                    cx,
                ));
            }
        }
    }

    div()
        .id("job-table")
        .size_full()
        .overflow_x_scroll()
        .border_1()
        .border_color(t.border)
        .rounded(px(8.))
        .bg(t.surface)
        .child(
            div()
                .flex()
                .flex_col()
                .min_w(px(MIN_WIDTH))
                .h_full()
                .child(header)
                .child(
                    div()
                        .id("job-rows")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .children(rows),
                ),
        )
}

fn job_row(job: &Job, cluster: Option<&str>, cx: &App) -> AnyElement {
    let t = theme(cx).clone();
    let hover = t.surface_hover;
    let text = |s: &str| SharedString::from(s.to_string());
    let values: [AnyElement; 10] = [
        mono(text(&job.id), cx).into_any_element(),
        text(&job.partition).into_any_element(),
        text(&job.name).into_any_element(),
        badge(&job.state, cx).into_any_element(),
        text(&job.time).into_any_element(),
        text(&job.nodes).into_any_element(),
        text(&job.reason).into_any_element(),
        text(&job.time_limit).into_any_element(),
        text(&job.cpus).into_any_element(),
        text(&job.min_memory).into_any_element(),
    ];

    let mut row = div()
        .flex()
        .h(px(34.))
        .border_b_1()
        .border_color(t.border)
        .hover(move |s| s.bg(hover));
    if let Some(name) = cluster {
        row = row.child(cell(&CLUSTER).child(text(name)));
    }
    row.children(COLUMNS.iter().zip(values).map(|(c, v)| cell(c).child(v)))
        .into_any_element()
}
