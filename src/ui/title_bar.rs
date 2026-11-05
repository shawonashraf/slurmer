//! The window frame slurmer draws itself when the compositor will not.
//!
//! GNOME on Wayland offers no server-side decorations, so gpui reports
//! `Decorations::Client` and the app must supply the title bar (drag area,
//! minimise / maximise / close) and the resize handles. Everywhere else
//! (macOS, Windows, X11, KDE) the system frame is used and none of this
//! renders.

use super::app_view::SlurmerApp;
use super::assets;
use super::model::{resize_edges, Edge, Tiled};
use super::theme::theme;
use super::widgets::icon;
use gpui::{
    div, prelude::*, px, AnyElement, Context, CursorStyle, FontWeight, MouseButton, Pixels,
    ResizeEdge, Tiling, Window,
};

const HEIGHT: Pixels = px(38.);
/// Width of the invisible strip along each free edge that starts a resize.
const GRIP: Pixels = px(6.);
/// Corners get a larger square so they are easy to hit.
const CORNER: Pixels = px(12.);

pub fn tiled(tiling: Tiling) -> Tiled {
    Tiled {
        top: tiling.top,
        bottom: tiling.bottom,
        left: tiling.left,
        right: tiling.right,
    }
}

/// Title bar with the app name and window buttons. Drag to move,
/// double-click to maximise, right-click for the compositor's window menu.
pub fn render(window: &Window, cx: &mut Context<SlurmerApp>) -> AnyElement {
    let t = theme(cx).clone();
    let controls = window.window_controls();
    let maximized = window.is_maximized();

    let mut buttons = div().flex().flex_1().justify_end().items_center().gap_2();
    if controls.minimize {
        buttons = buttons.child(control("minimize", assets::MINIMIZE, cx, |window| {
            window.minimize_window()
        }));
    }
    if controls.maximize {
        let path = if maximized {
            assets::RESTORE
        } else {
            assets::MAXIMIZE
        };
        buttons = buttons.child(control("maximize", path, cx, |window| window.zoom_window()));
    }
    buttons = buttons.child(control("close", assets::CLOSE, cx, |window| {
        window.remove_window()
    }));

    div()
        .id("title-bar")
        .flex()
        .flex_none()
        .items_center()
        .h(HEIGHT)
        .px_2()
        .bg(t.surface)
        .border_b_1()
        .border_color(t.border)
        // Start the compositor move only once the pointer moves, so a
        // double-click still reaches on_click.
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, _, _| this.title_drag = true),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _, _, _| this.title_drag = false),
        )
        .on_mouse_move(cx.listener(|this, _, window, _| {
            if this.title_drag {
                this.title_drag = false;
                window.start_window_move();
            }
        }))
        .on_click(|e, window, _| {
            if e.click_count() == 2 {
                window.zoom_window();
            }
        })
        .when(controls.window_menu, |bar| {
            bar.on_mouse_down(MouseButton::Right, |e, window, _| {
                window.show_window_menu(e.position)
            })
        })
        .child(div().flex_1())
        .child(
            div()
                .flex_none()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Slurmer"),
        )
        .child(buttons)
        .into_any_element()
}

/// A round GNOME-style window button.
fn control(
    id: &'static str,
    path: &'static str,
    cx: &mut Context<SlurmerApp>,
    action: fn(&mut Window),
) -> impl IntoElement {
    let t = theme(cx).clone();
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(24.))
        .rounded_full()
        .bg(t.surface_2)
        .cursor_pointer()
        .hover(move |s| s.bg(t.border))
        // Keep the press from starting a title-bar drag.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            action(window);
        })
        .child(icon(path, px(14.), t.text))
}

/// Invisible strips along the free edges that hand resizing to the
/// compositor. Drawn last so they sit above the content.
pub fn resize_handles(tiling: Tiling) -> Vec<AnyElement> {
    resize_edges(tiled(tiling))
        .into_iter()
        .map(|edge| {
            let (resize, cursor) = match edge {
                Edge::Top => (ResizeEdge::Top, CursorStyle::ResizeUpDown),
                Edge::Bottom => (ResizeEdge::Bottom, CursorStyle::ResizeUpDown),
                Edge::Left => (ResizeEdge::Left, CursorStyle::ResizeLeftRight),
                Edge::Right => (ResizeEdge::Right, CursorStyle::ResizeLeftRight),
                Edge::TopLeft => (ResizeEdge::TopLeft, CursorStyle::ResizeUpLeftDownRight),
                Edge::BottomRight => (ResizeEdge::BottomRight, CursorStyle::ResizeUpLeftDownRight),
                Edge::TopRight => (ResizeEdge::TopRight, CursorStyle::ResizeUpRightDownLeft),
                Edge::BottomLeft => (ResizeEdge::BottomLeft, CursorStyle::ResizeUpRightDownLeft),
            };
            let handle = div().absolute().cursor(cursor).occlude();
            let handle = match edge {
                Edge::Top => handle.top_0().left_0().right_0().h(GRIP),
                Edge::Bottom => handle.bottom_0().left_0().right_0().h(GRIP),
                Edge::Left => handle.top_0().bottom_0().left_0().w(GRIP),
                Edge::Right => handle.top_0().bottom_0().right_0().w(GRIP),
                Edge::TopLeft => handle.top_0().left_0().size(CORNER),
                Edge::TopRight => handle.top_0().right_0().size(CORNER),
                Edge::BottomLeft => handle.bottom_0().left_0().size(CORNER),
                Edge::BottomRight => handle.bottom_0().right_0().size(CORNER),
            };
            handle
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    cx.stop_propagation();
                    window.start_window_resize(resize);
                })
                .into_any_element()
        })
        .collect()
}
