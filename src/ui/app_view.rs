//! Root view: owns the store, the fetch results and the open form, and
//! lays out the sidebar, main panel and modal.

use super::cluster_form::{ClusterForm, FormEvent};
use super::model::{after_delete, initial_selection, Fetches};
use super::theme::{theme, Theme};
use super::{main_panel, sidebar};
use crate::config::{Cluster, Selection};
use crate::fetch::fetch_jobs;
use crate::ssh_config::SshHost;
use crate::store::{
    apply_delete, apply_save, apply_selection, list_ssh_hosts, validate_input, AppState,
    ClusterInput,
};
use gpui::{div, prelude::*, px, Context, Entity, MouseButton, Subscription, Window};
use std::path::PathBuf;
use uuid::Uuid;

pub struct SlurmerApp {
    state: AppState,
    pub clusters: Vec<Cluster>,
    pub ssh_hosts: Vec<SshHost>,
    pub selection: Selection,
    pub fetches: Fetches,
    pub load_error: Option<String>,
    form: Option<(Entity<ClusterForm>, Subscription)>,
    _appearance: Subscription,
}

impl SlurmerApp {
    pub fn new(config_path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = AppState::new(config_path);
        let (clusters, saved) = {
            let file = state.file.lock().expect("config lock");
            (file.clusters.clone(), file.selected.clone())
        };
        let load_error = state.load_error.lock().expect("config lock").clone();
        let selection = initial_selection(saved.as_ref(), &clusters);

        let appearance = cx.observe_window_appearance(window, |_, window, cx| {
            cx.set_global(Theme::for_appearance(window.appearance()));
            window.refresh();
        });

        Self {
            state,
            clusters,
            ssh_hosts: list_ssh_hosts(),
            selection,
            fetches: Fetches::default(),
            load_error,
            form: None,
            _appearance: appearance,
        }
    }

    pub fn select(&mut self, selection: Selection, cx: &mut Context<Self>) {
        // Persisting the selection is a convenience; failing is not fatal.
        let _ = apply_selection(&self.state, Some(selection.clone()));
        self.selection = selection;
        cx.notify();
    }

    pub fn refresh(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if self.fetches.is_loading(id) {
            return;
        }
        // Copy what ssh needs out of the lock; it is not held while ssh runs.
        let target = {
            let file = self.state.file.lock().expect("config lock");
            file.find(id).map(|c| (c.host.clone(), c.user.clone()))
        };
        let Some((host, user)) = target else {
            return;
        };
        self.fetches.start(id);
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { fetch_jobs(&host, &user) })
                .await;
            // Fails only if the window has closed; nothing left to update.
            let _ = this.update(cx, |app, cx| {
                app.fetches.finish(id, result, &app.clusters);
                cx.notify();
            });
        })
        .detach();
    }

    pub fn refresh_selected(&mut self, cx: &mut Context<Self>) {
        match self.selection.clone() {
            Selection::All => {
                let ids: Vec<Uuid> = self.clusters.iter().map(|c| c.id).collect();
                for id in ids {
                    self.refresh(id, cx);
                }
            }
            Selection::Cluster { id } => self.refresh(id, cx),
        }
    }

    pub fn dismiss_load_error(&mut self, cx: &mut Context<Self>) {
        self.load_error = None;
        cx.notify();
    }

    pub fn open_form(
        &mut self,
        cluster: Option<Cluster>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let hosts = self.ssh_hosts.clone();
        let form = cx.new(|cx| ClusterForm::new(cluster, hosts, window, cx));
        let sub = cx.subscribe(&form, Self::on_form_event);
        self.form = Some((form, sub));
        cx.notify();
    }

    fn close_form(&mut self, cx: &mut Context<Self>) {
        self.form = None;
        cx.notify();
    }

    fn on_form_event(
        &mut self,
        form: Entity<ClusterForm>,
        event: &FormEvent,
        cx: &mut Context<Self>,
    ) {
        let result = match event {
            FormEvent::Cancel => Ok(()),
            FormEvent::Save(input) => self.save(input.clone()),
            FormEvent::Delete(id) => self.delete(*id, cx),
        };
        match result {
            Ok(()) => self.close_form(cx),
            Err(message) => form.update(cx, |f, cx| f.set_error(message, cx)),
        }
    }

    fn save(&mut self, input: ClusterInput) -> Result<(), String> {
        let cluster = validate_input(input)?;
        apply_save(&self.state, cluster.clone())?;
        match self.clusters.iter_mut().find(|c| c.id == cluster.id) {
            Some(existing) => *existing = cluster,
            None => self.clusters.push(cluster),
        }
        // A successful save rewrote the file, so a stale load error no longer applies.
        self.load_error = None;
        Ok(())
    }

    fn delete(&mut self, id: Uuid, cx: &mut Context<Self>) -> Result<(), String> {
        apply_delete(&self.state, id)?;
        self.clusters.retain(|c| c.id != id);
        self.fetches.forget(id);
        if let Some(next) = after_delete(&self.selection, id, &self.clusters) {
            self.select(next, cx);
        }
        Ok(())
    }
}

impl Render for SlurmerApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx).clone();
        let form = self.form.as_ref().map(|(form, _)| form.clone());
        div()
            .relative()
            .flex()
            .size_full()
            .bg(t.bg)
            .text_color(t.text)
            .text_size(px(13.))
            .child(sidebar::render(self, cx))
            .child(main_panel::render(self, cx))
            .when_some(form, |root, form| {
                root.child(
                    div()
                        .id("backdrop")
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(t.scrim)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.close_form(cx)),
                        )
                        .child(form),
                )
            })
    }
}
