# Ticket Flow Dashboard

Ticket Flow Dashboard is a desktop app built with Tauri 2. It shows your tickets from Jira and GitHub issues in one place, with the ticket-flow runs on your machine and the state of their merge requests on GitHub or GitLab. It is read-only. It never starts a run, never writes to a ticket and never changes a merge request. Read `dashboard/PRODUCT.md` for the product goals and the design spec at `docs/superpowers/specs/2026-10-26-ticket-flow-dashboard-design.md` for the full behaviour.

## Prerequisites

- Node 22 and pnpm 11.
- Rust (stable) with cargo.
- On Linux, the Tauri system packages: `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev`.
- `gh` (GitHub) or `glab` (GitLab) installed and logged in, depending on where your repositories live.
- `acli` (Atlassian CLI) logged in with `acli jira auth login --web`, for Jira projects.
- On macOS, an app opened from Finder or the Dock does not get your shell's PATH. The app also looks for `gh`, `glab` and `acli` in `/opt/homebrew/bin` and `/usr/local/bin`, where Homebrew puts them. Installed anywhere else, they are not found.

## First launch

The app opens on the settings the first time.

Type your root folders, one per line. The app looks for repositories up to three folder levels below each root. A root may start with `~`, which stands for your home folder (`~/work` for example).

Jira needs no setting, only the `acli` login. Your Jira projects must live on the site `acli` is logged in to, shown by `acli jira auth status` and changed with `acli jira auth switch`.

If you used an earlier version of the app, you can delete its keychain entry under the service `ticket-flow-dashboard` and revoke its token at id.atlassian.com.

Each source is optional.

- Without `acli` logged in, Jira projects show your local runs only. GitHub projects work fully.
- With no `gh`, GitHub projects show local runs but no issues, and no merge request state on GitHub.
- With no `glab`, runs in GitLab repositories show without their merge request state.

The sidebar says which source is missing or failing.

A blank overview means the app found no repository under your roots. A repository counts only when it holds `.claude/ticket-flow.md`. Check the roots, then make sure the plugin is set up in each repository.

## Run in dev

```sh
cd dashboard/app
pnpm install
pnpm tauri dev
```

## Run the tests

```sh
cd dashboard/app
pnpm test
cd src-tauri
cargo test
```

## Cut a release

The release version comes from `tauri.conf.json`, not from the tag. First set the same version in `dashboard/app/package.json`, `dashboard/app/src-tauri/tauri.conf.json` and `dashboard/app/src-tauri/Cargo.toml`. Then tag `dashboard-v<that version>` and push the tag.

```sh
git tag dashboard-v0.1.0
git push origin dashboard-v0.1.0
```

The `dashboard release` workflow runs the tests and builds on Ubuntu, Windows and macOS (universal). It creates a draft GitHub release with the installers. Check the files, then publish the draft by hand. You can also start the workflow by hand from the Actions tab or with `gh workflow run`, but only once the workflow file is on the default branch. Before the merge, push a tag to test it.

## Unsigned builds

The installers are not signed or notarised. Each OS warns you on first launch.

On macOS, Gatekeeper says the app is damaged or from an unidentified developer. Right-click the app, choose Open, then confirm. If that is not enough, run `xattr -dr com.apple.quarantine "/Applications/Ticket Flow Dashboard.app"`.

On Windows, SmartScreen shows "Windows protected your PC". Click More info, then Run anyway.

On Linux, there is no warning. Make an AppImage executable with `chmod +x` before you run it.
