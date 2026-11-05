# gpui Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Tauri v2 + Svelte 5 frontend with a native gpui UI, keeping every behaviour, the ssh call, the config file location and the release/Homebrew flow.

**Architecture:** One Rust binary crate at the repo root. The existing pure backend modules (`slurm`, `ssh_config`, `config`, `fetch`, and `commands` renamed to `store`) stay as they are minus Tauri glue. A new `ui/` module holds a pure, unit-tested view model (`ui/model.rs`) and gpui views built from plain `div()` elements and a hand-written text input. ssh runs on gpui's background executor via `std::process::Command::output()`; tokio is dropped.

**Tech Stack:** Rust 2021, `gpui = "0.2.2"` (official, crates.io; no component library), `serde`/`serde_json`, `uuid`, `dirs`, `unicode-segmentation` (text input). Packaging: `cargo-packager`.

**Spec:** `docs/superpowers/specs/2026-09-14-slurmer-design.md` (behaviour), this plan (stack change). User decisions 2026-11-05: official gpui only, hand-built widgets; cargo-packager for bundles.

## Global Constraints

- ssh argv unchanged: `ssh [-o User=<user>] -- <host> squeue --noheader (-u '<user>' | --me) -o '<SQUEUE_FORMAT>'`. No BatchMode, ConnectTimeout, retry, polling.
- Config file stays at `<dirs::config_dir()>/org.shawonashraf.slurmer/clusters.json` (same path Tauri's `app_config_dir` used on all three OSes), so existing users keep their clusters.
- Malformed-`clusters.json` guard, `-`-prefixed host rejection (backend and form), atomic save: unchanged.
- No mutex guard held while ssh runs: copy host/user out of the lock before spawning.
- Window: title `slurmer`, 1100×680, min 900×500, resizable, app id `org.shawonashraf.slurmer`. Quit when the window closes.
- Colours: the light/dark tokens from the old `src/app.css`, chosen by `window.appearance()` and updated on appearance change.
- Bundle names the Homebrew workflow downloads: `slurmer_<ver>_aarch64.dmg`, `slurmer_<ver>_x64.dmg`. Bundle id `org.shawonashraf.slurmer`, `.app` name `slurmer.app`.
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` clean at every commit. No test fixtures with personal hostnames.

## Review Focus

1. Closing the window while a fetch runs: the task's `WeakEntity::update` fails; must be ignored, not panic.
2. Deleting a cluster while its fetch is in flight: the late result must not resurrect its entry in `jobs`/`errors`. `model::Fetches::finish` drops results for unknown ids (test in Task 2).
3. Saved selection pointing at a deleted cluster: falls back to the first cluster, else All (`model::initial_selection` tests).
4. Non-ASCII in the text input (IME, emoji, paste with newlines): input keeps UTF-8/UTF-16 offset handling from the gpui example; paste flattens newlines.
5. Very long ssh stderr (256 KiB): error strip must scroll, not blow up layout. The strip goes in a `max_h` + `overflow_y_scroll` container.

---

### Task 1: Move the crate to the repo root and drop Tauri

**Files:**
- Move: `src-tauri/{Cargo.toml,Cargo.lock,src/*.rs,icons/}` → repo root `Cargo.toml`, `Cargo.lock`, `src/`, `assets/icons/`
- Rename: `src/commands.rs` → `src/store.rs`
- Delete: `src/` Svelte files, `index.html`, `package*.json`, `svelte.config.js`, `tsconfig*.json`, `vite.config.ts`, `src-tauri/{build.rs,tauri.conf.json,capabilities,.gitignore}`, `src/lib.rs`
- Modify: `src/fetch.rs`, `src/store.rs`, `src/slurm.rs` (drop `Serialize` on `Job`), `src/main.rs`, `.gitignore`

**Interfaces:**
- Produces: `store::AppState::new(path: PathBuf) -> AppState` with `path`, `file: Mutex<ClustersFile>`, `load_error: Mutex<Option<String>>`; `store::{ClusterInput, validate_input, apply_save, apply_delete, apply_selection}` unchanged; `store::list_ssh_hosts() -> Vec<SshHost>`; `fetch::fetch_jobs(host: &str, user: &str) -> Result<Vec<Job>, FetchError>` (now sync); `main::config_path() -> PathBuf`.

- [ ] **Step 1:** `git mv` the backend and icons; delete the frontend and Tauri files. Cargo.toml: `[[bin]] name = "slurmer"`, drop `[lib]`, `tauri`, `tauri-build`, `tokio`; add `gpui = "0.2.2"`, `dirs = "6"`, `unicode-segmentation = "1"`. Keep `[profile.release]` but `panic = "abort"` stays.
- [ ] **Step 2:** `fetch.rs`: `fetch_jobs_with(program, host, user)` becomes sync using `std::process::Command::output()` (still drains both pipes concurrently; keep `CREATE_NO_WINDOW` via `std::os::windows::process::CommandExt`). Tests: same seven cases, `#[test]` instead of `#[tokio::test]`, no `.await`.
- [ ] **Step 3:** `store.rs`: delete `#[tauri::command]` wrappers and `ClustersResponse`; move the `list_ssh_hosts` body into a plain `pub fn list_ssh_hosts() -> Vec<SshHost>`. Existing tests untouched.
- [ ] **Step 4:** `main.rs`: keep `windows_subsystem`; declare modules; `config_path()` = `$SLURMER_CONFIG` if set, else `dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("org.shawonashraf.slurmer").join("clusters.json")`; temporary `main` opens an empty gpui window (replaced in Task 4).
- [ ] **Step 5:** Verify: `cargo test` (all previous backend tests pass), `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- [ ] **Step 6:** Commit "Move the crate to the repo root and drop Tauri and the Svelte frontend".

### Task 2: Pure view model (TDD)

**Files:** Create `src/ui/mod.rs`, `src/ui/model.rs`.

**Interfaces:**
- Produces:
  - `enum Tone { Green, Amber, Red, Blue, Grey }`, `fn tone_for(state: &str) -> Tone` (sets from `jobState.ts`, trimmed + uppercased).
  - `fn initial_selection(saved: Option<&Selection>, clusters: &[Cluster]) -> Selection`.
  - `struct Fetches { jobs: HashMap<Uuid, Vec<Job>>, errors: HashMap<Uuid, String>, loading: HashSet<Uuid>, fetched: HashSet<Uuid> }` with `start(id)`, `finish(id, Result<Vec<Job>, FetchError>, known: &[Cluster])`, `forget(id)`, `any_loading() -> bool`, `status(id) -> DotStatus` (`Loading | Error | Ok | Idle`).
  - `struct Group<'a> { cluster: &'a Cluster, jobs: &'a [Job], error: Option<&'a str>, fetched: bool }`, `fn groups<'a>(sel: &Selection, clusters: &'a [Cluster], f: &'a Fetches) -> Vec<Group<'a>>` (error only set in the All view).
  - `enum Body { NoClusters, NotRefreshed, Empty, ErrorOnly, Table }`, `fn body(sel: &Selection, clusters: &[Cluster], f: &Fetches) -> Body` — the `MainPanel.svelte` decision order.
  - `fn subtitle(sel: &Selection, clusters: &[Cluster]) -> String` ("1 cluster"/"N clusters"/"user@host"/"host"/"").
  - `fn after_delete(sel: &Selection, deleted: Uuid, remaining: &[Cluster]) -> Option<Selection>`.

- [ ] **Step 1: failing tests** in `model.rs`: `tone_for(" running ") == Green`, `"OUT_OF_MEMORY" → Red`, `"COMPLETED" → Blue`, `"WEIRD" → Grey`; `initial_selection` returns saved cluster if present, `All` if saved All, first cluster if saved id missing, `All` if no clusters; `finish` for an id not in `known` leaves `jobs`/`errors` empty and clears `loading`; success clears a previous error; error keeps previous jobs; `body` returns `NoClusters`, `NotRefreshed` before any fetch, `Empty` for a single cluster fetched with zero jobs, `ErrorOnly` for single error with no rows, `Table` otherwise; `groups` in single view has `error: None` even when errored; `subtitle` cases; `after_delete` picks first remaining or All, `None` when another cluster was selected.
- [ ] **Step 2:** `cargo test ui::model` fails to compile/fail.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `cargo test ui::model` passes.
- [ ] **Step 5:** Commit "Add a pure view model for the gpui UI".

### Task 3: Theme, widgets and text input

**Files:** Create `src/ui/theme.rs`, `src/ui/widgets.rs`, `src/ui/text_input.rs`, `src/ui/assets.rs`, `assets/icons/{refresh,edit}.svg`.

**Interfaces:**
- Produces:
  - `struct Theme { bg, surface, surface_2, surface_hover, text, muted, border, accent, accent_hover, accent_text, green, green_bg, amber, amber_bg, red, red_bg, blue, blue_bg, grey, grey_bg: Hsla, mono: SharedString }` with `Theme::light()`, `Theme::dark()`, `Theme::for_appearance(WindowAppearance)`; `impl Global`; `fn theme(cx: &App) -> &Theme`. Hex values copied from old `app.css`.
  - `widgets::button(id, label, Variant { Default, Primary, Ghost, Danger }, disabled, cx) -> Stateful<Div>`; `badge(state: &str, cx) -> Div`; `dot(color: Hsla) -> Div`; `pulse(id) -> impl IntoElement` (loading indicator, opacity animation); `strip(kind: StripKind { Error, Warn }, cx) -> Div`.
  - `TextInput` entity: `TextInput::new(cx, initial: &str, placeholder: &str) -> Self`, `text(&self) -> &str`, `set_text(&mut self, s: &str, cx)`, `Focusable`. Actions in namespace `text_input` bound under key context `"TextInput"`; `text_input::bind_keys(cx)` registers cmd- (macOS) / ctrl- (others) bindings. Emits nothing; parent reads `text()` on save.
  - `Assets` implementing `AssetSource` for `icons/refresh.svg`, `icons/edit.svg` via `include_bytes!`.
- [ ] **Step 1:** Port `examples/input.rs` from gpui 0.2.2 into `TextInput` (no let-chains: crate is edition 2021), colours from `Theme`, 30px height, border, focus ring in accent.
- [ ] **Step 2:** Unit test in `text_input.rs` for the pure offset helpers only (`previous_boundary`/`next_boundary` over `"aé😀"`), extracted as free functions over `&str`.
- [ ] **Step 3:** `cargo test`, clippy, fmt. Commit "Add theme tokens, basic widgets and a text input for gpui".

### Task 4: Main window: sidebar, main panel, job table, refresh

**Files:** Create `src/ui/app_view.rs` (root `SlurmerApp`), `src/ui/sidebar.rs`, `src/ui/main_panel.rs`, `src/ui/job_table.rs`. Modify `src/main.rs`.

**Interfaces:**
- Consumes: Task 1 `store`, `fetch`; Task 2 `model`; Task 3 widgets/theme.
- Produces: `SlurmerApp { state: Arc<AppState>, clusters: Vec<Cluster>, ssh_hosts: Vec<SshHost>, selection: Selection, fetches: Fetches, load_error: Option<String>, form: Option<Entity<ClusterForm>> }`; methods `select(&mut self, Selection, cx)`, `refresh(&mut self, id: Uuid, cx)`, `refresh_selected(cx)`, `open_form(Option<Cluster>, window, cx)`, `close_form(cx)`. Sidebar/main panel/table are render functions taking `&SlurmerApp` and `&mut Context<SlurmerApp>`.
- [ ] **Step 1:** `refresh`: `fetches.start(id)`; copy host/user under the lock; `cx.spawn(async move |this, cx| { let r = cx.background_executor().spawn(async move { fetch_jobs(&host, &user) }).await; this.update(cx, |app, cx| { app.fetches.finish(id, r, &app.clusters); cx.notify(); }).ok(); }).detach()`. `refresh_selected` loops over every cluster for All.
- [ ] **Step 2:** Sidebar (220px): brand, "All clusters" item, "Clusters" section, per-cluster row with status dot / pulse, name, edit icon on hover; "+ Add cluster" button. Main panel: title/subtitle toolbar + Refresh/Refreshing button, dismissible load-error strip, single-view error strip with the `ssh <host>` hint, then `model::body` branches. Table: header row + rows in a vertically scrolling `div`, column widths from `JobTable.svelte` (Cluster 120, Job ID 100, numeric 90 right-aligned, Reason 140, Name flex), min width 900, group notes for error/not-refreshed/empty.
- [ ] **Step 3:** `main.rs`: `Application::new().with_assets(Assets).run(...)`: set `Theme` global, bind keys, macOS menu with Quit (`cmd-q`), open window with constraints above, observe appearance → swap theme global and `cx.refresh_windows()`, `cx.on_window_closed(|cx| cx.quit())`.
- [ ] **Step 4:** Verify: `SLURMER_CONFIG` env var overrides `config_path()` (debug aid, also used for manual testing so the real config is never touched); run with a temp `clusters.json` and a fake `ssh` script first on `PATH` that prints two jobs for one host and fails for another. Screenshot the window (`screencapture -l`) and inspect light and dark. Commit "Build the gpui main window: sidebar, job table and refresh".

### Task 5: Cluster form modal

**Files:** Create `src/ui/cluster_form.rs`. Modify `src/ui/app_view.rs`.

**Interfaces:**
- Produces: `ClusterForm::new(cluster: Option<Cluster>, ssh_hosts: Vec<SshHost>, window, cx) -> Self`; emits `FormEvent::{Save(ClusterInput), Delete(Uuid), Cancel}` via `EventEmitter`; `set_error(&mut self, msg: String, cx)`. `SlurmerApp` subscribes and does the store calls itself, so the form never touches `AppState`; on error it calls `set_error`, on success it closes the form.
- [ ] **Step 1:** Fields Name (autofocus), Host (help text), User, optional "Import from ssh config" picker (button toggling an inline scrollable list; picking fills host, user if set, name if blank). `can_save` = name and host non-blank, host not starting with `-`. Escape → cancel, Enter → save (`key_context("ClusterForm")`). Edit mode: "Delete cluster" → inline "Delete this cluster? [Delete] [Keep]". Backdrop click cancels; modal click does not.
- [ ] **Step 2:** `SlurmerApp` save: `validate_input` → `apply_save` → upsert into `clusters`, clear `load_error`; delete: `apply_delete` → remove, `fetches.forget`, `after_delete` → `select`. Selection changes call `apply_selection` and ignore its error.
- [ ] **Step 3:** Manual acceptance (spec's five steps) with config dir backed up: add via ssh import, refresh, rename, delete, relaunch. Screenshot each. Commit "Add the cluster form modal with ssh config import and delete".

### Task 6: Packaging and CI

**Files:** Modify `Cargo.toml` (`[package.metadata.packager]`), `.github/workflows/release.yml`. Create `.github/workflows/ci.yml`.

- [ ] **Step 1:** Packager config: `product-name = "slurmer"`, `identifier = "org.shawonashraf.slurmer"`, `icons` from `assets/icons`, `formats` per OS (macOS `app`,`dmg`; Linux `deb`,`appimage`; Windows `nsis`), `before-packaging-command = "cargo build --release"` (with target passthrough), `macos.minimum-system-version = "11.0"`.
- [ ] **Step 2:** `cargo install cargo-packager --locked` locally, `cargo packager --release`; confirm `slurmer_0.1.0_aarch64.dmg` name and that `slurmer.app` launches. If the name differs, set the packager's file-name template or rename in the workflow.
- [ ] **Step 3:** `release.yml`: drop Node and tauri-action; per matrix entry install Rust target, Linux gpui deps (`libxkbcommon-x11-dev libwayland-dev libx11-xcb-dev libfontconfig-dev libvulkan-dev`), `cargo install cargo-packager --locked`, `cargo packager --release --target …`, `gh release upload <tag> <files>`. `ci.yml` on pull_request: fmt, clippy, test on macOS/Linux/Windows.
- [ ] **Step 4:** `actionlint` if available. Commit "Package with cargo-packager and add PR CI".

### Task 7: Docs

- [ ] README (stack line, Develop/Build/Release sections), CLAUDE.md (commands, architecture, invariants for gpui), spec: add a "2026-11-05: UI moved to gpui" section superseding the Svelte rows. Commit "Document the gpui stack".

Then: push branch (ask first), open PR.
