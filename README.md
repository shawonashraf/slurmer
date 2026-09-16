# slurmer

A small desktop app that shows your Slurm job queue on one or more HPC
clusters, fetched over ssh with a manual Refresh. Cross-platform port of
[Job Tracker](https://github.com/shawonashraf/jobtracker) built with Rust,
Tauri v2 and Svelte 5.

![screenshot](sc.png)

## Install

### macOS via Homebrew

```bash
brew install --cask shawonashraf/tap/slurmer
```

Needs macOS 11 or newer. Release builds are ad-hoc signed and not notarized;
the cask removes the quarantine attribute from `slurmer.app` after installing,
so the app opens without the "damaged" Gatekeeper dialog and no manual `xattr`
step is needed. Later:

```bash
brew upgrade --cask slurmer     # move to the latest release
brew uninstall --cask slurmer   # remove the app, keep your saved clusters
brew uninstall --zap --cask slurmer   # also delete clusters.json and caches
```

### Other platforms and manual installs

Download a bundle from the [releases page](https://github.com/shawonashraf/slurmer/releases):
`.dmg` for macOS, `.deb`/`.rpm`/`.AppImage` for Linux, `.msi`/`.exe` for
Windows. On macOS, clear the quarantine flag after copying the app to
`/Applications`:

```bash
xattr -cr /Applications/slurmer.app
```

## How it works

- Saved cluster profiles (name, ssh host, user) live in `clusters.json` under
  the app's config directory.
- Refresh runs the system `ssh` binary:
  `ssh -o User=<user> <host> squeue --noheader -u '<user>' -o '<format>'`.
  The host is used verbatim, so aliases from `~/.ssh/config` apply, including
  `ControlMaster` sockets and any 2FA session you already opened in a terminal.
- When User is left blank, the app omits `-o User=` and runs `squeue --me`
  instead of `-u`, so the alias's own `User` line applies; `--me` needs
  Slurm 20.02 or newer.
- Output is parsed into a table with colour-coded job states.

If ssh needs an interactive login, open a terminal and run `ssh <host>` once,
then press Refresh again.

## Requirements

- An `ssh` client on your PATH (built in on macOS, Linux, and Windows 10+).
- ssh access to a Slurm cluster.

## Develop

```bash
npm install
npm run tauri dev      # run the app with hot reload
npm run check          # svelte-check + tsc
cd src-tauri && cargo test && cargo clippy --all-targets
```

## Build

```bash
npm run tauri build    # bundles for the current platform into src-tauri/target/release/bundle/
```

## Release

1. Bump `version` in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and
   `package.json`, commit, and push a matching `v*` tag.
2. The `release` workflow builds bundles for macOS (arm64 and x64), Linux and
   Windows and attaches them to a draft GitHub release.
3. Publish the draft. That triggers the `homebrew` workflow, which downloads
   the two macOS dmgs, renders the cask with `scripts/homebrew-cask.sh`, and
   pushes it to
   [shawonashraf/homebrew-tap](https://github.com/shawonashraf/homebrew-tap).
   The cask in the tap is generated output; change the script, not the tap.
