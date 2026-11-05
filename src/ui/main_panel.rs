//! Right pane: toolbar with Refresh, notices, and the empty/error/table body.

use super::app_view::SlurmerApp;
use super::assets;
use super::job_table;
use super::model::{body, groups, selected_cluster, subtitle, title, Body};
use super::theme::theme;
use super::widgets::{button, icon, mono, spinner, strip, StripKind, Variant};
use crate::config::Selection;
use gpui::{div, prelude::*, px, AnyElement, App, Context, Div, FontWeight, SharedString};

pub fn render(app: &SlurmerApp, cx: &mut Context<SlurmerApp>) -> impl IntoElement {
    let t = theme(cx).clone();
    let is_all = app.selection == Selection::All;
    let selected = selected_cluster(&app.selection, &app.clusters);
    let busy = match selected {
        _ if is_all => app.fetches.any_loading(),
        Some(c) => app.fetches.is_loading(c.id),
        None => false,
    };
    let sub = subtitle(&app.selection, &app.clusters);

    let mut titles = div().flex().flex_col().min_w_0().child(
        div()
            .text_size(px(18.))
            .font_weight(FontWeight::BOLD)
            .child(title(&app.selection, &app.clusters)),
    );
    if !sub.is_empty() {
        titles = titles.child(mono(sub.clone(), cx).mt(px(2.)).text_color(t.muted));
    }

    let mut toolbar = div()
        .flex()
        .items_start()
        .justify_between()
        .gap_3()
        .child(titles);
    if !app.clusters.is_empty() {
        let accent_text = t.accent_text;
        let mut refresh = button("refresh", Variant::Primary, busy, cx);
        refresh = if busy {
            refresh
                .child(spinner("refresh-spin", px(14.), accent_text))
                .child("Refreshing")
        } else {
            refresh
                .child(icon(assets::REFRESH, px(14.), accent_text))
                .child("Refresh")
                .on_click(cx.listener(|this, _, _, cx| this.refresh_selected(cx)))
        };
        toolbar = toolbar.child(refresh);
    }

    let mut panel = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .h_full()
        .px(px(20.))
        .py_4()
        .gap_3()
        .child(toolbar);

    if let Some(err) = &app.load_error {
        panel = panel.child(
            strip(StripKind::Warn, cx)
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(div().flex_1().child(SharedString::from(err.clone())))
                .child(
                    button("dismiss", Variant::Ghost, false, cx)
                        .child("Dismiss")
                        .on_click(cx.listener(|this, _, _, cx| this.dismiss_load_error(cx))),
                ),
        );
    }

    if let Some(c) = selected {
        if let Some(err) = app.fetches.errors.get(&c.id) {
            panel = panel.child(error_block("single-error", None, err, &c.host, cx));
        }
    }

    let content: AnyElement = match body(&app.selection, &app.clusters, &app.fetches) {
        Body::NoClusters => empty(cx)
            .child(heading("No clusters yet"))
            .child("Add a cluster to start tracking its Slurm queue.")
            .child(
                button("empty-add", Variant::Primary, false, cx)
                    .child("Add cluster")
                    .on_click(cx.listener(|this, _, window, cx| this.open_form(None, window, cx))),
            )
            .into_any_element(),
        Body::NotRefreshed => empty(cx)
            .child(heading("Press Refresh to load jobs"))
            .child(if is_all {
                "Every cluster is refreshed in parallel.".to_string()
            } else {
                sub
            })
            .into_any_element(),
        Body::ErrorOnly => div().into_any_element(),
        Body::Empty => empty(cx)
            .child(heading("No jobs in the queue"))
            .child("Nothing running or pending for this user.")
            .into_any_element(),
        Body::Table => job_table::render(
            &groups(&app.selection, &app.clusters, &app.fetches),
            is_all,
            cx,
        )
        .into_any_element(),
    };

    panel.child(
        div()
            .flex_1()
            .min_h_0()
            .when(busy, |d| d.opacity(0.55))
            .child(content),
    )
}

fn empty(cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.))
        .size_full()
        .text_color(theme(cx).muted)
}

fn heading(text: &'static str) -> impl IntoElement {
    div()
        .text_size(px(15.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(text)
}

/// Error strip with the ssh output and the "open a terminal" hint. Long
/// output (a chatty ssh banner, a huge stderr) scrolls inside the strip.
pub fn error_block(
    id: impl Into<SharedString>,
    cluster_name: Option<&str>,
    error: &str,
    host: &str,
    cx: &App,
) -> Div {
    let t = theme(cx).clone();
    let id: SharedString = id.into();
    let mut out = strip(StripKind::Error, cx).flex().flex_col().gap_1();
    if let Some(name) = cluster_name {
        out = out.child(
            div()
                .font_weight(FontWeight::BOLD)
                .child(SharedString::from(format!("{name}:"))),
        );
    }
    out.child(
        div()
            .id(id)
            .max_h(px(160.))
            .overflow_y_scroll()
            .child(mono(SharedString::from(error.to_string()), cx)),
    )
    .child(
        div()
            .flex()
            .flex_wrap()
            .items_baseline()
            .gap_1()
            .text_size(px(12.))
            .text_color(t.muted)
            .child("Open a terminal and run")
            .child(mono(SharedString::from(format!("ssh {host}")), cx))
            .child("once to establish a session, then retry."),
    )
}
