# Ticket Flow Dashboard

Ticket Flow Dashboard is a desktop app built with Tauri 2. It shows your tickets from Jira, GitHub and GitLab in one place and lets you start and follow ticket-flow runs on them. Read `dashboard/PRODUCT.md` for the product goals and the design spec for the full behaviour.

## Prerequisites

- Node 22 and pnpm 11.
- Rust (stable) with cargo.
- On Linux, the Tauri system packages: `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev`.
- `gh` (GitHub) or `glab` (GitLab) installed and logged in, depending on your trackers.
- On Linux, a Secret Service such as GNOME Keyring or KWallet. The app stores the Jira token there.

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

Set the same version in `dashboard/app/package.json`, `dashboard/app/src-tauri/tauri.conf.json` and `dashboard/app/src-tauri/Cargo.toml`. Then tag and push.

```sh
git tag dashboard-v0.1.0
git push origin dashboard-v0.1.0
```

The `dashboard release` workflow runs the tests and builds on Ubuntu, Windows and macOS (universal). It creates a draft GitHub release with the installers. Check the files, then publish the draft by hand. You can also start the workflow from the Actions tab.

## Unsigned builds

The installers are not signed or notarised. Each OS warns you on first launch.

On macOS, Gatekeeper says the app is damaged or from an unidentified developer. Right-click the app, choose Open, then confirm. If that is not enough, run `xattr -dr com.apple.quarantine "/Applications/Ticket Flow Dashboard.app"`.

On Windows, SmartScreen shows "Windows protected your PC". Click More info, then Run anyway.

On Linux, there is no warning. Make an AppImage executable with `chmod +x` before you run it.
