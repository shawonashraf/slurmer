# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

slurmer is a Tauri v2 desktop app (Rust backend, Svelte 5 + TypeScript frontend) that shows a user's Slurm job queue on one or more saved HPC clusters. It shells out to the system `ssh` binary on a manual Refresh; there is no polling, no notifications, no `scancel`, and no in-app SSH library by design. Design spec: `docs/superpowers/specs/2026-09-14-slurmer-design.md` (the binding authority when code and intent disagree).

## Commands

```bash
npm install                       # once; also installs @tauri-apps/cli
npm run tauri dev                 # run the app with hot reload (Vite on 127.0.0.1:1420)
npm run check                     # svelte-check + tsc; the only frontend typecheck/lint gate
npm run tauri build               # platform bundles into src-tauri/target/release/bundle/
npm run tauri build -- --no-bundle  # just the release binary (faster end-to-end compile check)

cd src-tauri
cargo test                        # all backend tests (fetch.rs tests are unix-only)
cargo test config::               # one module (slurm, ssh_config, config, fetch, commands)
cargo test shell_quote            # tests whose name contains a substring
cargo clippy --all-targets        # must be warning-free
cargo fmt --check                 # crate is kept rustfmt-clean
```

Run `npm` commands from the repo root and `cargo` commands from `src-tauri/`. The first `cargo` build compiles Tauri and takes several minutes. Releases are cut by pushing a `v*` tag; `.github/workflows/release.yml` builds unsigned bundles for macOS (both arches), Linux and Windows as a draft release.

## Architecture

**The Rust backend owns all non-visual logic; the frontend is a thin view over six Tauri commands.** Nothing in `src/` parses squeue output, reads files, or spawns processes.

Backend (`src-tauri/src/`), one concern per module, all pure or path-parameterised so `cargo test` needs no cluster:

- `slurm.rs`: `Job` (ten string fields), `SQUEUE_FORMAT`, `parse_squeue_output`, `shell_quote`, `build_ssh_args`.
- `ssh_config.rs`: lists single-name `Host` aliases from `~/.ssh/config` with `HostName`/`User`, for the form's import dropdown. Duplicate aliases merge first-wins.
- `config.rs`: `Cluster`, `Selection` (`{"kind":"all"}` / `{"kind":"cluster","id":…}`), `ClustersFile` load/save/upsert/remove/find. Save is atomic (temp file + rename).
- `fetch.rs`: spawns `ssh` with `tokio::process::Command::output()` (drains stdout and stderr concurrently). `fetch_jobs_with(program, …)` exists so tests can point it at a fake ssh shell script.
- `commands.rs`: `AppState` (config path, `Mutex<ClustersFile>`, `Mutex<Option<String>>` load error) plus the commands `list_clusters`, `save_cluster`, `delete_cluster`, `set_selection`, `list_ssh_hosts`, `fetch_jobs`. Logic lives in testable `validate_input` / `apply_save` / `apply_delete` / `apply_selection`; the `#[tauri::command]` functions are one-line wrappers.
- `lib.rs`: registers the commands and manages `AppState` at `<app_config_dir>/clusters.json` (on macOS `~/Library/Application Support/org.shawonashraf.slurmer/`).

Frontend (`src/`): `lib/api.ts` has the typed `invoke` wrappers and shared types; `lib/state.svelte.ts` exports the `app` singleton (Svelte 5 runes class) holding clusters, ssh hosts, selection, and per-cluster `jobs` / `errors` / `loading` / `fetched` maps keyed by cluster id, so switching clusters shows the last fetch without a new ssh call. `App.svelte` is the two-pane shell and owns the modal state; `MainPanel.svelte` decides which empty/error/table state to show; `JobTable.svelte` renders `Group[]` (from `lib/jobState.ts`, which also holds the state→badge colour map).

### The ssh call

```
ssh [-o User=<user>] -- <host> squeue --noheader (-u '<user>' | --me) -o '<SQUEUE_FORMAT>'
```

`host` is the profile's host verbatim so `~/.ssh/config` aliases (ControlMaster, ProxyJump, 2FA sessions already opened in a terminal) apply. No `BatchMode`, `ConnectTimeout`, retry, or hardcoded alias: these were explicit decisions, do not add them. A blank user drops `-o User=` and uses `squeue --me`.

## Invariants to preserve

- **Shell safety is two-layered.** Anything interpreted by the remote shell (`-u` value, format string) goes through `shell_quote`. The local argv is never run through a shell, and `--` separates options from the host so a `-`-prefixed host cannot become an ssh option; `validate_input` and the form's `canSave` both reject such hosts. Keep all three when touching `build_ssh_args` or the form.
- **A malformed `clusters.json` is never overwritten until the user saves a cluster.** While `load_error` is set, `apply_delete` refuses and `apply_selection` updates memory only; `apply_save` clears the error after writing. Tests assert the on-disk bytes stay untouched.
- **No mutex guard across an `.await`.** `fetch_jobs` copies host/user out of the lock before awaiting.
- **Command arguments are snake_case** (`#[tauri::command(rename_all = "snake_case")]`); `api.ts` passes `{ cluster_id }`, `{ input }`, etc. Keep both sides in step.
- **Errors cross IPC as strings or `{ message }`**; `errorMessage()` in `api.ts` normalises both. Non-zero ssh exit reports trimmed stderr, or `ssh exited with status N`.
- **Frontend is Svelte 5 runes only** (`$state`, `$derived`, `$props`), no stores, no component library, no Tailwind, no Tauri plugins. Styling is plain CSS on the tokens in `src/app.css` (light on `:root`, dark under `prefers-color-scheme: dark`). `ClusterForm.svelte` snapshots its `cluster` prop into `$state` on purpose; `App.svelte` remounts it on every open, so keep that `{#if formOpen}` pattern.
- **Native `window.confirm`/`alert` do not render in the webview**; confirmations are in-app (see the two-step delete in the form).

## Testing conventions

- Backend tests sit in each module's `#[cfg(test)] mod tests`, use `tempfile` for filesystem cases, and use generic fixtures only: never put personal hostnames or usernames in tests (`snellius` / `snellius_user` style is fine).
- `fetch.rs` tests are `#[cfg(all(test, unix))]` and drive real processes through a fake `ssh` script written to a temp dir; extend those rather than mocking.
- There are no frontend unit tests by design; `npm run check` is the gate. The five-step manual flow (add via ssh import, refresh, rename, delete, relaunch) is the UI acceptance check.
