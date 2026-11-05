//! Add/edit cluster modal. It only collects input and emits events; the
//! root view does the store calls and reports errors back via `set_error`.

use super::assets;
use super::text_input::TextInput;
use super::theme::theme;
use super::widgets::{button, icon, mono, strip, StripKind, Variant};
use crate::config::Cluster;
use crate::ssh_config::SshHost;
use crate::store::ClusterInput;
use gpui::{
    actions, div, prelude::*, px, AnyElement, App, Context, Entity, EventEmitter, FocusHandle,
    Focusable, FontWeight, KeyBinding, MouseButton, SharedString, Window,
};
use uuid::Uuid;

actions!(cluster_form, [Cancel, Submit, FocusNext, FocusPrev]);

const CONTEXT: &str = "ClusterForm";

pub fn bind_keys(cx: &mut App) {
    let ctx = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, ctx),
        KeyBinding::new("enter", Submit, ctx),
        KeyBinding::new("tab", FocusNext, ctx),
        KeyBinding::new("shift-tab", FocusPrev, ctx),
    ]);
}

pub enum FormEvent {
    Save(ClusterInput),
    Delete(Uuid),
    Cancel,
}

/// Same rule as `store::validate_input`, checked live to enable Save.
pub fn can_save(name: &str, host: &str) -> bool {
    let host = host.trim();
    !name.trim().is_empty() && !host.is_empty() && !host.starts_with('-')
}

pub struct ClusterForm {
    focus_handle: FocusHandle,
    cluster: Option<Cluster>,
    ssh_hosts: Vec<SshHost>,
    name: Entity<TextInput>,
    host: Entity<TextInput>,
    user: Entity<TextInput>,
    import_open: bool,
    confirming_delete: bool,
    error: Option<String>,
}

impl EventEmitter<FormEvent> for ClusterForm {}

impl Focusable for ClusterForm {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ClusterForm {
    pub fn new(
        cluster: Option<Cluster>,
        ssh_hosts: Vec<SshHost>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let field = |value: Option<&String>, placeholder: &str, cx: &mut Context<Self>| {
            let initial = value.map(String::as_str).unwrap_or("");
            let input = cx.new(|cx| TextInput::new(cx, initial, placeholder));
            // Re-render on every edit so Save enables and disables live.
            cx.observe(&input, |_, _, cx| cx.notify()).detach();
            input
        };
        let name = field(cluster.as_ref().map(|c| &c.name), "Snellius", cx);
        let host = field(
            cluster.as_ref().map(|c| &c.host),
            "ssh alias or hostname",
            cx,
        );
        let user = field(
            cluster.as_ref().map(|c| &c.user),
            "leave blank to use the alias's User",
            cx,
        );
        window.focus(&name.focus_handle(cx));

        Self {
            focus_handle: cx.focus_handle(),
            cluster,
            ssh_hosts,
            name,
            host,
            user,
            import_open: false,
            confirming_delete: false,
            error: None,
        }
    }

    pub fn set_error(&mut self, message: String, cx: &mut Context<Self>) {
        self.error = Some(message);
        cx.notify();
    }

    fn values(&self, cx: &App) -> (String, String, String) {
        (
            self.name.read(cx).text().to_string(),
            self.host.read(cx).text().to_string(),
            self.user.read(cx).text().to_string(),
        )
    }

    fn submit(&mut self, _: &Submit, _: &mut Window, cx: &mut Context<Self>) {
        let (name, host, user) = self.values(cx);
        if !can_save(&name, &host) {
            return;
        }
        self.error = None;
        cx.emit(FormEvent::Save(ClusterInput {
            id: self.cluster.as_ref().map(|c| c.id),
            name,
            host,
            user,
        }));
    }

    fn cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(FormEvent::Cancel);
    }

    fn focus_next(&mut self, _: &FocusNext, window: &mut Window, _: &mut Context<Self>) {
        window.focus_next();
    }

    fn focus_prev(&mut self, _: &FocusPrev, window: &mut Window, _: &mut Context<Self>) {
        window.focus_prev();
    }

    fn apply_import(&mut self, alias: &SshHost, cx: &mut Context<Self>) {
        self.host.update(cx, |i, cx| i.set_text(&alias.alias, cx));
        if let Some(user) = &alias.user {
            self.user.update(cx, |i, cx| i.set_text(user, cx));
        }
        if self.name.read(cx).text().trim().is_empty() {
            self.name.update(cx, |i, cx| i.set_text(&alias.alias, cx));
        }
        self.import_open = false;
        cx.notify();
    }

    fn import_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = theme(cx).clone();
        let hover = t.surface_hover;
        let mut picker = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(field_label("Import from ssh config", cx))
            .child(
                div()
                    .id("import")
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(30.))
                    .px_2()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if self.import_open { t.accent } else { t.border })
                    .bg(t.surface)
                    .cursor_pointer()
                    .text_color(t.muted)
                    .child("Choose an alias…")
                    .child(icon(assets::CHEVRON, px(14.), t.muted))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.import_open = !this.import_open;
                        cx.notify();
                    })),
            );
        if self.import_open {
            let items = self.ssh_hosts.iter().enumerate().map(|(i, h)| {
                let label = match &h.hostname {
                    Some(hn) => format!("{} ({hn})", h.alias),
                    None => h.alias.clone(),
                };
                let host = h.clone();
                div()
                    .id(("import-item", i))
                    .px_2()
                    .py_1p5()
                    .cursor_pointer()
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .hover(move |s| s.bg(hover))
                    .child(SharedString::from(label))
                    .on_click(cx.listener(move |this, _, _, cx| this.apply_import(&host, cx)))
            });
            picker = picker.child(
                div()
                    .id("import-list")
                    .max_h(px(160.))
                    .overflow_y_scroll()
                    .py_1()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(t.border)
                    .bg(t.surface)
                    .children(items),
            );
        }
        picker.into_any_element()
    }
}

fn field_label(text: &'static str, cx: &App) -> impl IntoElement {
    div()
        .text_size(px(12.))
        .text_color(theme(cx).muted)
        .child(text)
}

impl Render for ClusterForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let (name, host, _) = self.values(cx);
        let savable = can_save(&name, &host);
        let is_edit = self.cluster.is_some();

        let mut form = div()
            .id("cluster-form")
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::focus_next))
            .on_action(cx.listener(Self::focus_prev))
            // Keep clicks inside the modal from reaching the backdrop.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .flex()
            .flex_col()
            .gap(px(14.))
            .w(px(420.))
            .p_5()
            .rounded(px(8.))
            .border_1()
            .border_color(t.border)
            .bg(t.surface)
            .shadow_lg()
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::BOLD)
                    .child(if is_edit {
                        "Edit cluster"
                    } else {
                        "Add cluster"
                    }),
            );

        if !self.ssh_hosts.is_empty() {
            form = form.child(self.import_picker(cx));
        }

        form = form
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(field_label("Name", cx))
                    .child(self.name.clone()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(field_label("Host", cx))
                    .child(self.host.clone())
                    .child(div().text_size(px(11.)).text_color(t.muted).child(
                        "Used verbatim as the ssh target, so aliases from ~/.ssh/config apply.",
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(field_label("User", cx))
                    .child(self.user.clone()),
            );

        if let Some(err) = &self.error {
            form = form.child(
                strip(StripKind::Error, cx).child(
                    div()
                        .id("form-error")
                        .max_h(px(120.))
                        .overflow_y_scroll()
                        .child(mono(SharedString::from(err.clone()), cx)),
                ),
            );
        }

        let mut actions = div().flex().items_center().gap_2().mt_1p5();
        if let Some(cluster) = &self.cluster {
            let id = cluster.id;
            actions = if self.confirming_delete {
                actions
                    .child(
                        div()
                            .text_color(t.red)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Delete this cluster?"),
                    )
                    .child(
                        button("confirm-delete", Variant::Danger, false, cx)
                            .child("Delete")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.error = None;
                                cx.emit(FormEvent::Delete(id));
                            })),
                    )
                    .child(
                        button("keep", Variant::Default, false, cx)
                            .child("Keep")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.confirming_delete = false;
                                cx.notify();
                            })),
                    )
            } else {
                actions.child(
                    button("delete", Variant::Ghost, false, cx)
                        .text_color(t.red)
                        .child("Delete cluster")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.confirming_delete = true;
                            cx.notify();
                        })),
                )
            };
        }
        actions = actions
            .child(div().flex_1())
            .child(
                button("cancel", Variant::Default, false, cx)
                    .child("Cancel")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(FormEvent::Cancel))),
            )
            .child(
                button("save", Variant::Primary, !savable, cx)
                    .child("Save")
                    .when(savable, |b| {
                        b.on_click(
                            cx.listener(|this, _, window, cx| this.submit(&Submit, window, cx)),
                        )
                    }),
            );

        form.child(actions)
    }
}

#[cfg(test)]
mod tests {
    use super::can_save;

    #[test]
    fn can_save_matches_validate_input() {
        assert!(can_save("n", "h"));
        assert!(can_save(" n ", " snellius "));
        assert!(!can_save("  ", "h"));
        assert!(!can_save("n", "  "));
        assert!(!can_save("n", " -oProxyCommand=x"));
    }
}
