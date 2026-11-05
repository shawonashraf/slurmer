//! Left pane: "All clusters", one row per cluster, and the add button.

use super::app_view::SlurmerApp;
use super::assets;
use super::model::DotStatus;
use super::theme::theme;
use super::widgets::{button, dot, icon, spinner, Variant};
use crate::config::{Cluster, Selection};
use gpui::{div, prelude::*, px, AnyElement, Context, FontWeight, SharedString};

pub fn render(app: &SlurmerApp, cx: &mut Context<SlurmerApp>) -> impl IntoElement {
    let t = theme(cx).clone();
    let is_all = app.selection == Selection::All;

    let all_item = item("all", is_all, cx)
        .child(
            div()
                .w(px(16.))
                .flex()
                .justify_center()
                .child(SharedString::from("▦")),
        )
        .child(label("All clusters"))
        .on_click(cx.listener(|this, _, _, cx| this.select(Selection::All, cx)));

    let mut nav = div()
        .id("nav")
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(2.))
        .overflow_y_scroll()
        .child(all_item)
        .child(
            div()
                .mt(px(14.))
                .mb_1()
                .mx_2()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t.muted)
                .child("CLUSTERS"),
        );

    if app.clusters.is_empty() {
        nav = nav.child(
            div()
                .px_2()
                .py_1p5()
                .text_size(px(12.))
                .text_color(t.muted)
                .child("No clusters yet"),
        );
    }
    for c in &app.clusters {
        nav = nav.child(cluster_row(app, c, cx));
    }

    div()
        .flex()
        .flex_col()
        .flex_none()
        .w(px(220.))
        .h_full()
        .p_3()
        .bg(t.surface_2)
        .border_r_1()
        .border_color(t.border)
        .child(
            div()
                .px_2()
                .pt_1p5()
                .pb(px(14.))
                .text_size(px(15.))
                .font_weight(FontWeight::BOLD)
                .child("slurmer"),
        )
        .child(nav)
        .child(
            button("add-cluster", Variant::Default, false, cx)
                .mt_3()
                .justify_center()
                .child("+ Add cluster")
                .on_click(cx.listener(|this, _, window, cx| this.open_form(None, window, cx))),
        )
}

fn item(
    id: impl Into<gpui::ElementId>,
    active: bool,
    cx: &mut Context<SlurmerApp>,
) -> gpui::Stateful<gpui::Div> {
    let t = theme(cx).clone();
    let hover = t.surface_hover;
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .min_w_0()
        .h(px(30.))
        .px_2()
        .rounded(px(6.))
        .cursor_pointer()
        .when(active, |d| d.bg(t.accent).text_color(t.accent_text))
        .when(!active, |d| d.hover(move |s| s.bg(hover)))
}

fn label(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .child(text.into())
}

fn cluster_row(app: &SlurmerApp, c: &Cluster, cx: &mut Context<SlurmerApp>) -> AnyElement {
    let t = theme(cx).clone();
    let active = app.selection == (Selection::Cluster { id: c.id });
    let fg = if active { t.accent_text } else { t.text };
    let status: AnyElement = match app.fetches.status(c.id) {
        DotStatus::Loading => {
            spinner(SharedString::from(format!("spin-{}", c.id)), px(10.), fg).into_any_element()
        }
        DotStatus::Error => dot(px(8.), t.red).into_any_element(),
        DotStatus::Ok => dot(px(8.), t.green).into_any_element(),
        DotStatus::Idle => dot(px(8.), t.border).into_any_element(),
    };
    let group: SharedString = format!("row-{}", c.id).into();
    let id = c.id;
    let edit_target = c.clone();

    item(SharedString::from(format!("cluster-{}", c.id)), active, cx)
        .group(group.clone())
        .child(div().w(px(16.)).flex().justify_center().child(status))
        .child(label(c.name.clone()))
        .child(
            div()
                .id(SharedString::from(format!("edit-{}", c.id)))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(22.))
                .rounded(px(4.))
                .opacity(if active { 0.8 } else { 0. })
                .group_hover(group, |s| s.opacity(1.))
                .hover(|s| s.bg(gpui::black().opacity(0.12)))
                .child(icon(assets::EDIT, px(14.), fg))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.open_form(Some(edit_target.clone()), window, cx);
                })),
        )
        .on_click(cx.listener(move |this, _, _, cx| this.select(Selection::Cluster { id }, cx)))
        .into_any_element()
}
