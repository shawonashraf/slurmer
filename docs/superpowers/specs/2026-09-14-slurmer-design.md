# slurmer — Design

## Purpose

slurmer is a cross-platform desktop app (macOS, Linux, Windows) that shows a
user's Slurm job queue on one or more HPC clusters, fetched over SSH with a
manual Refresh. It is a Rust + Tauri v2 port of the macOS SwiftUI
[Job Tracker](https://github.com/shawonashraf/jobtracker), with a polished UI
and support for multiple saved clusters.

## Decisions

| Question | Decision |
|---|---|
| SSH mechanism | Shell out to the system `ssh` binary. Inherits `~/.ssh/config`, ControlMaster sockets, ssh-agent, ProxyJump, and 2FA sessions already opened in a terminal. |
| Refresh | Manual only. No polling, no notifications. |
| Clusters | A saved list of cluster profiles. Sidebar shows "All clusters" plus one entry per cluster. |
| Hardcoded aliases | None. The original's `Snellius-Large` special-case is dropped. The profile's `host` is the ssh target verbatim. |
| Main view | Styled table, one row per job, colour-coded state badges, light/dark theme. |
| Stack | Rust backend does all non-visual work. Svelte 5 + TypeScript + plain CSS frontend. |

## Architecture

Standard Tauri v2 layout: `src-tauri/` for Rust, `src/` for Svelte.

### Rust backend (`src-tauri/src/`)

| Module | Responsibility |
|---|---|
| `config.rs` | `Cluster { id: Uuid, name, host, user }` and `ClustersFile { clusters: Vec<Cluster>, selected: Option<Selection> }`. Load/save `clusters.json` in the platform config dir (via the `directories` crate, e.g. `~/Library/Application Support/slurmer/clusters.json` on macOS). Missing file → empty list. |
| `ssh_config.rs` | Parse `~/.ssh/config` into `Vec<SshHost { alias, hostname: Option<String>, user: Option<String> }>`. Keys are case-insensitive. Comments and blank lines skipped. `Host` lines with multiple patterns or wildcards are skipped. Missing file → empty list. |
| `slurm.rs` | `Job` struct with the ten squeue fields; `SQUEUE_FORMAT` (see below); `parse_squeue_output(&str) -> Vec<Job>`; `shell_quote(&str) -> String`; `build_ssh_args(host, user) -> Vec<String>`. Pure functions. |
| `fetch.rs` | `async fn fetch_jobs(host, user) -> Result<Vec<Job>, FetchError>`. Spawns `ssh` with `tokio::process::Command`, drains stdout and stderr concurrently with `tokio::join!`, maps non-zero exit to `FetchError`. On Windows sets `CREATE_NO_WINDOW`. |
| `commands.rs` | Tauri commands: `list_clusters`, `save_cluster`, `delete_cluster`, `set_selection`, `list_ssh_hosts`, `fetch_jobs(cluster_id)`. Config lives in Tauri managed state as `Mutex<ClustersFile>`. |

`Job` fields, identical to the original: `id`, `partition`, `name`, `state`,
`time`, `nodes`, `reason`, `time_limit`, `cpus`, `min_memory`. All strings.

`Selection` is an enum: `All` or `Cluster(Uuid)`.

The squeue format string, unchanged from the original:

```
%i|%P|%j|%T|%M|%D|%R|%l|%C|%m
```

### Svelte frontend (`src/`)

| File | Responsibility |
|---|---|
| `App.svelte` | Two-pane layout: sidebar plus main panel. Boots by calling `list_clusters` and `list_ssh_hosts`. |
| `lib/Sidebar.svelte` | "All clusters" entry, one entry per cluster, an add button. Hover actions for edit and delete. |
| `lib/JobTable.svelte` | Styled table. Props: rows, `showCluster` boolean for the combined view. |
| `lib/StateBadge.svelte` | Colour-coded state pill. |
| `lib/ClusterForm.svelte` | Add/edit modal: Name, Host, User fields and an "Import from ssh config" dropdown. |
| `lib/api.ts` | Typed `invoke()` wrappers for each command. |
| `lib/state.svelte.ts` | Svelte 5 runes state: clusters, ssh hosts, selection, per-cluster jobs, per-cluster error, per-cluster loading flag. |
| `app.css` | Design tokens (light and dark), reset, shared styles. |

## Data flow

**Startup.** Frontend calls `list_clusters` and `list_ssh_hosts`. Backend
loads `clusters.json`. Sidebar restores the saved `selected` entry if it still
exists, otherwise the first cluster, otherwise "All clusters" with the
empty-state prompt to add one. Every selection change calls `set_selection`.

**Refresh, single cluster.** Frontend calls `fetch_jobs(cluster_id)`. Backend
looks up the cluster, builds the argv, spawns `ssh`, parses stdout, returns
`Vec<Job>` or `FetchError`. Frontend stores the result keyed by cluster id, so
switching clusters shows the last fetched data without a new ssh call.

**Refresh, all clusters.** Frontend calls `fetch_jobs` once per cluster with
`Promise.allSettled`. Each cluster's rows and error are stored independently.
The combined table adds a leading Cluster column and groups rows by cluster in
sidebar order. A cluster whose fetch failed shows an inline error strip in
place of its rows.

**The ssh call.**

```
ssh -o User=<user> <host> squeue --noheader -u '<user>' -o '<SQUEUE_FORMAT>'
```

`host` is the profile's host verbatim: an alias from `~/.ssh/config` or a raw
hostname. `-o User=` makes the profile's username win over the alias's `User`
line. The `-u` value and the format string are wrapped by `shell_quote`
(single quotes, embedded `'` becomes `'\''`) because the remote shell
interprets them. No `BatchMode`, no `ConnectTimeout`, no retry, matching the
original's explicit choice.

**Parsing.** Split stdout on newlines, split each line on `|`, keep only lines
with exactly ten fields.

## Visual design

**Layout.** Fixed-width sidebar (~220px) on the left. Main panel: a toolbar
row with the cluster name as heading, host and user in muted text beneath,
Refresh button on the right with a spinner while loading. Below: error strip
if any, then the table filling the remaining height with its own scroll.

**Table.** Sticky header. Subtle row hover, hairline dividers, no zebra
stripes. Column order as the original: Job ID, Partition, Name, State, Time,
Nodes, Reason, Time Limit, CPUs, Min Memory (plus a leading Cluster column in
the combined view). Job ID in monospace. Name takes flexible width and
truncates with an ellipsis and a title tooltip. Nodes, CPUs, Min Memory
right-aligned. Time and Time Limit use tabular figures.

**State badge.** Rounded pill with a dot.

| Slurm state | Colour |
|---|---|
| RUNNING, COMPLETING | green |
| PENDING, CONFIGURING, SUSPENDED | amber |
| FAILED, CANCELLED, TIMEOUT, NODE_FAIL, OUT_OF_MEMORY | red |
| COMPLETED | blue |
| anything else | neutral grey |

**Theme.** CSS custom properties for background, surface, text, muted text,
border, accent, and the four badge colours. Light values on `:root`, dark
values under `prefers-color-scheme: dark`. System UI font stack. No component
library, no Tailwind.

**States.**

- No clusters: centred prompt with an "Add cluster" button.
- Cluster never fetched: "Press Refresh to load jobs".
- Fetch succeeded with zero rows: "No jobs in the queue".
- Loading: spinner in the Refresh button; the existing table dims instead of
  clearing.
- Fetch error: error strip with the trimmed stderr and the hint
  "Open a terminal and run `ssh <host>` once to establish a session, then
  retry."

**Cluster form.** Modal with Name, Host, User fields and an "Import from ssh
config" dropdown listing aliases. Choosing an alias fills Host with the alias
and User with its `User` line. Save disabled until Name and Host are
non-empty. Delete asks for confirmation.

## Error handling

- Non-zero ssh exit: trimmed stderr, or "ssh exited with status N" if stderr
  is empty.
- ssh binary cannot be spawned: "Could not run ssh: <os error>".
- Unreadable or malformed `clusters.json`: reported once in the UI; the app
  continues with an empty list and does not overwrite the file until the user
  saves a cluster.
- Missing `~/.ssh/config`: not an error; import dropdown is empty.
- Exit code zero with empty stdout: no jobs, shown as the empty state.

## Testing

All with `cargo test`, no cluster required.

- `slurm.rs`: multi-line parse; malformed lines skipped; untruncated names
  preserved; `shell_quote` with embedded single quotes; `build_ssh_args`
  yields the exact argv.
- `ssh_config.rs`: multiple Host blocks; comments and blank lines;
  case-insensitive keys; missing User line; multi-pattern `Host` lines
  skipped.
- `config.rs`: save/load round trip in a temp dir; malformed file not
  overwritten on load; delete of the selected cluster clears selection.
- No tests for the live ssh call or Svelte views. `cargo clippy` and
  `npm run check` serve as lint and typecheck.

## Packaging

- Tauri v2 bundles: `.dmg`/`.app` (macOS), `.deb`/`.AppImage` (Linux),
  `.msi`/`.exe` (Windows). On Windows the app calls `ssh.exe` from the
  built-in OpenSSH client.
- GitHub Actions workflow using `tauri-apps/tauri-action` builds all three on
  tag push. Unsigned; macOS users strip quarantine with `xattr -cr`.
- App identifier `org.shawonashraf.slurmer`.

## Out of scope

Auto-refresh, desktop notifications, `scancel`, job detail views, in-app
password or 2FA prompts, a native SSH library.
