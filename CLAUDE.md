# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

slurmer is a native Rust desktop app with a [gpui](https://gpui.rs) UI that shows a user's Slurm job queue on one or more saved HPC clusters. It shells out to the system `ssh` binary on a manual Refresh; there is no polling, no notifications, no `scancel`, and no in-app SSH library by design. Design spec: `docs/superpowers/specs/2026-09-14-slurmer-design.md` (the binding authority when code and intent disagree; its 2026-11-05 amendment covers the move from Tauri + Svelte to gpui).

## Commands

```bash
cargo run                         # debug build of the app
SLURMER_CONFIG=/tmp/c.json cargo run   # use a scratch clusters.json, not the real one
cargo test                        # all tests (fetch.rs tests are unix-only)
cargo test ui::model              # one module (slurm, ssh_config, config, fetch, store, ui::model, ...)
cargo test shell_quote            # tests whose name contains a substring
cargo clippy --all-targets -- -D warnings   # must be warning-free
cargo fmt --check                 # crate is kept rustfmt-clean

cargo install cargo-packager --locked
cargo build --release --no-default-features && cargo packager --release --formats app,dmg   # deb,appimage on Linux; nsis,wix on Windows
```

The first build compiles gpui and takes several minutes. On macOS the default `runtime-shaders` feature makes gpui compile its Metal shaders at app start (about 0.2 s on first launch, cached by macOS after), so local builds do not need Xcode's separately downloaded Metal Toolchain. Release builds use `--no-default-features` and precompile the shaders; building that way locally needs `xcodebuild -downloadComponent MetalToolchain`. Linux builds need `libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-xcb-dev libxcb1-dev libfontconfig1-dev libfreetype6-dev libvulkan-dev`.

`.github/workflows/ci.yml` runs fmt, clippy, tests and a debug cargo-packager pass on macOS, Linux and Windows for every PR. Releases are cut by pushing a `v*` tag; `.github/workflows/release.yml` builds unsigned bundles with cargo-packager for macOS (both arches), Linux and Windows as a draft release. Publishing that release triggers `.github/workflows/homebrew.yml`, which renders `Casks/slurmer.rb` for `shawonashraf/homebrew-tap` from `scripts/homebrew-cask.sh` (version plus per-arch dmg sha256) and pushes it over the `HOMEBREW_TAP_DEPLOY_KEY` secret. The cask downloads `Slurmer_<ver>_aarch64.dmg` / `Slurmer_<ver>_x64.dmg` and installs `Slurmer.app`; these come from `product-name = "Slurmer"` in `Cargo.toml`, so changing it means changing `scripts/homebrew-cask.sh` and `homebrew.yml` too. The executable is `Slurmer` (`[[bin]]`); the Debian package name stays lowercase `slurmer`. The cask in the tap is generated output: change the script, not the tap file.

Fedora RPMs are built on COPR (`shawonashraf/slurmer`) from a GitHub webhook on tag push. COPR runs `.copr/Makefile`, which calls `packaging/fedora/make-srpm.sh`: it packs `HEAD` with `git archive`, vendors all crates (`cargo vendor`) plus a `LICENSE.dependencies` list into a second tarball, copies `Cargo.toml`'s version into `packaging/fedora/slurmer.spec`, and runs `rpmbuild -bs`. The RPM build itself is offline (`cargo build --frozen`). The RPM installs the binary as lowercase `/usr/bin/slurmer`, plus `packaging/linux/org.shawonashraf.slurmer.desktop` (named after `APP_ID` so Wayland matches the window to its icon) and hicolor icons from `assets/icons/`.

## Architecture

One binary crate at the repo root. Backend modules (`src/`) are pure or path-parameterised so `cargo test` needs no cluster or display:

- `slurm.rs`: `Job` (ten string fields), `SQUEUE_FORMAT`, `parse_squeue_output`, `shell_quote`, `build_ssh_args`.
- `ssh_config.rs`: lists single-name `Host` aliases from `~/.ssh/config` with `HostName`/`User`, for the form's import picker. Duplicate aliases merge first-wins.
- `config.rs`: `Cluster`, `Selection` (`{"kind":"all"}` / `{"kind":"cluster","id":…}`), `ClustersFile` load/save/upsert/remove/find. Save is atomic (temp file + rename).
- `fetch.rs`: runs `ssh` with `std::process::Command::output()` (drains stdout and stderr concurrently). Synchronous; the UI calls it on gpui's background executor. `fetch_jobs_with(program, …)` exists so tests can point it at a fake ssh shell script.
- `store.rs`: `AppState` (config path, `Mutex<ClustersFile>`, `Mutex<Option<String>>` load error), `validate_input` / `apply_save` / `apply_delete` / `apply_selection`, `list_ssh_hosts`.
- `main.rs`: `config_path()` (`$SLURMER_CONFIG`, else `dirs::config_dir()/org.shawonashraf.slurmer/clusters.json`, the same path the old Tauri build used), key bindings, macOS menu, window (1100×680, min 900×500), quit on window close.

UI (`src/ui/`):

- `model.rs`: pure view rules with unit tests: `tone_for` (state → badge colour), `initial_selection`, `after_delete`, `Fetches` (per-cluster `jobs` / `errors` / `loading` / `fetched` keyed by cluster id, so switching clusters shows the last fetch without a new ssh call), `groups`, `body` (which empty/error/table state the main panel shows), `title` / `subtitle`.
- `app_view.rs`: `SlurmerApp`, the root entity. Owns `AppState`, a mirror of the cluster list, `Fetches`, and the open `ClusterForm`. Does every store call. `refresh` spawns one background task per cluster.
- `sidebar.rs`, `main_panel.rs`, `job_table.rs`: render functions over `&SlurmerApp`.
- `cluster_form.rs`: modal entity. Only collects input and emits `FormEvent::{Save, Delete, Cancel}`; the root view calls the store and reports errors back with `set_error`. A new form entity is created on every open.
- `title_bar.rs`: the self-drawn window frame (title, round minimise/maximise/close, drag, double-click to maximise, right-click window menu, invisible resize strips). Renders only when `window.window_decorations()` is `Client`. `main.rs` requests `WindowDecorations::Client` on Linux because gpui 0.2.2 otherwise asks for server decorations and reports `Server` even on GNOME Wayland, which draws none. `main.rs` also calls `set_window_title` after opening, since gpui applies `TitlebarOptions::title` only on X11.
- `text_input.rs`: single-line input adapted from gpui 0.2.2's `examples/input.rs` (IME, UTF-16 offsets, clipboard).
- `widgets.rs` (button, badge, spinner, strip, mono text), `theme.rs` (light/dark tokens as a gpui `Global`, swapped on window appearance change), `assets.rs` (SVG icons from `assets/ui/` compiled in via `include_bytes!`).

### The ssh call

```
ssh [-o User=<user>] -- <host> squeue --noheader (-u '<user>' | --me) -o '<SQUEUE_FORMAT>'
```

`host` is the profile's host verbatim so `~/.ssh/config` aliases (ControlMaster, ProxyJump, 2FA sessions already opened in a terminal) apply. No `BatchMode`, `ConnectTimeout`, retry, or hardcoded alias: these were explicit decisions, do not add them. A blank user drops `-o User=` and uses `squeue --me`.

## Invariants to preserve

- **Shell safety is two-layered.** Anything interpreted by the remote shell (`-u` value, format string) goes through `shell_quote`. The local argv is never run through a shell, and `--` separates options from the host so a `-`-prefixed host cannot become an ssh option; `validate_input` and the form's `can_save` both reject such hosts. Keep all three when touching `build_ssh_args` or the form.
- **A malformed `clusters.json` is never overwritten until the user saves a cluster.** While `load_error` is set, `apply_delete` refuses and `apply_selection` updates memory only; `apply_save` clears the error after writing. Tests assert the on-disk bytes stay untouched.
- **No config lock held while ssh runs.** `SlurmerApp::refresh` copies host/user out of the lock before spawning the background task.
- **A fetch result for a cluster deleted mid-fetch is dropped** (`Fetches::finish` takes the known cluster list). A task whose window has closed ignores the failed `update`.
- **Official gpui only.** `gpui = "0.2.2"` from crates.io, no component library (gpui-component and the `gpui-pre` snapshots were considered and rejected). Widgets are plain `div()` builders on the `Theme` tokens.
- **The form never touches the store.** Keep store calls in `SlurmerApp` so validation and the malformed-file guard live in one place.
- **Keep the config path.** Changing `APP_ID` or `config_path()` strands existing users' clusters.

## Testing conventions

- Tests sit in each module's `#[cfg(test)] mod tests`, use `tempfile` for filesystem cases, and use generic fixtures only: never put personal hostnames or usernames in tests (`snellius` / `snellius_user` style is fine).
- `fetch.rs` tests are `#[cfg(all(test, unix))]` and drive real processes through a fake `ssh` script written to a temp dir; extend those rather than mocking.
- View decisions belong in `ui/model.rs` with unit tests; gpui render code is not unit-tested. The five-step manual flow (add via ssh import, refresh, rename, delete, relaunch), run with `SLURMER_CONFIG` pointing at a scratch file, is the UI acceptance check.
