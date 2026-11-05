//! Small styled building blocks shared by the views.

use super::assets;
use super::model::{tone_for, Tone};
use super::theme::{theme, Theme};
use gpui::{
    div, percentage, prelude::*, px, svg, Animation, AnimationExt as _, App, Div, ElementId, Hsla,
    Pixels, SharedString, Stateful, Svg, Transformation,
};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Default,
    Primary,
    Ghost,
    Danger,
}

/// A 30px button shell. Add children and, unless `disabled`, an `on_click`.
pub fn button(
    id: impl Into<ElementId>,
    variant: Variant,
    disabled: bool,
    cx: &App,
) -> Stateful<Div> {
    let t = theme(cx);
    let (bg, border, fg, hover) = match variant {
        Variant::Default => (t.surface, t.border, t.text, t.surface_hover),
        Variant::Primary => (t.accent, t.accent, t.accent_text, t.accent_hover),
        Variant::Ghost => (transparent(), transparent(), t.text, t.surface_hover),
        Variant::Danger => (transparent(), t.red, t.red, t.red_bg),
    };
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap_1p5()
        .h(px(30.))
        .px_3()
        .border_1()
        .border_color(border)
        .rounded(px(6.))
        .bg(bg)
        .text_color(fg)
        .whitespace_nowrap()
        .when(disabled, |b| b.opacity(0.55))
        .when(!disabled, |b| {
            b.cursor_pointer().hover(move |s| s.bg(hover))
        })
}

pub fn transparent() -> Hsla {
    gpui::transparent_black()
}

pub fn icon(path: &'static str, size: Pixels, color: Hsla) -> Svg {
    svg().path(path).size(size).flex_none().text_color(color)
}

/// A rotating arc, used while ssh runs.
pub fn spinner(id: impl Into<ElementId>, size: Pixels, color: Hsla) -> impl IntoElement {
    icon(assets::SPINNER, size, color).with_animation(
        id,
        Animation::new(Duration::from_millis(700)).repeat(),
        |svg, delta| svg.with_transformation(Transformation::rotate(percentage(delta))),
    )
}

pub fn dot(size: Pixels, color: Hsla) -> Div {
    div().size(size).flex_none().rounded_full().bg(color)
}

pub fn tone_colors(tone: Tone, t: &Theme) -> (Hsla, Hsla) {
    match tone {
        Tone::Green => (t.green, t.green_bg),
        Tone::Amber => (t.amber, t.amber_bg),
        Tone::Red => (t.red, t.red_bg),
        Tone::Blue => (t.blue, t.blue_bg),
        Tone::Grey => (t.grey, t.grey_bg),
    }
}

/// Colour-coded job state pill.
pub fn badge(state: &str, cx: &App) -> Div {
    let (fg, bg) = tone_colors(tone_for(state), theme(cx));
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap_1p5()
        .h(px(20.))
        .px_2()
        .rounded_full()
        .bg(bg)
        .text_color(fg)
        .text_size(px(11.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .whitespace_nowrap()
        .child(dot(px(6.), fg))
        .child(SharedString::from(state.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripKind {
    Error,
    Warn,
}

/// Coloured message box for errors and warnings.
pub fn strip(kind: StripKind, cx: &App) -> Div {
    let t = theme(cx);
    let (fg, bg) = match kind {
        StripKind::Error => (t.red, t.red_bg),
        StripKind::Warn => (t.amber, t.amber_bg),
    };
    div().px_3().py_2().rounded(px(6.)).bg(bg).text_color(fg)
}

/// Monospace text, used for ssh output and hosts.
pub fn mono(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .font_family(theme(cx).mono.clone())
        .text_size(px(12.))
        .child(text.into())
}
