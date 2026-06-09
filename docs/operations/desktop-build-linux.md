# Desktop build — Linux artifact (D1)

Produces an installable Linux package of the desktop app from source. This is the first
deliverable of the [executable delivery plan](./executable-delivery-plan.md) (D1): a runnable
Linux artifact from a local build.

## Prerequisites

System libraries required by Tauri 2 on Linux (Debian/Ubuntu names):

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev librsvg2-dev libssl-dev build-essential
```

Plus a Rust toolchain (`x86_64-unknown-linux-gnu` target) and Node.js with the desktop-app
dependencies installed (`cd desktop-app && npm install`).

## Build

```bash
cd desktop-app
npm run tauri build
```

This runs the frontend build (`tsc && vite build`), compiles the Rust binary in release mode,
and bundles three artifacts under `target/release/bundle/`:

| Artifact | Path |
|----------|------|
| Debian package | `target/release/bundle/deb/Alpen Multisig_<version>_amd64.deb` |
| RPM package | `target/release/bundle/rpm/Alpen Multisig-<version>-1.x86_64.rpm` |
| AppImage | `target/release/bundle/appimage/Alpen Multisig_<version>_amd64.AppImage` |

The AppImage step downloads `linuxdeploy` helpers on first run, so the initial build needs
network access.

## Install & launch

**AppImage (no install, double-click or one command):**

```bash
chmod +x "Alpen Multisig_<version>_amd64.AppImage"
"./Alpen Multisig_<version>_amd64.AppImage"
```

**Debian package:**

```bash
sudo apt install "./Alpen Multisig_<version>_amd64.deb"
# launches from the app menu, or:
desktop-app
```

The app expects the orchestrator backend on `http://localhost:3000` (see the
[runbook](./runbook.md)).

## Icons

Bundle icons are generated from the Alpen mark and committed under
`desktop-app/src-tauri/icons/`. The source is `desktop-app/src-tauri/icon-source.svg`
(the `AlpenMark` polygon on a white background). To regenerate after a brand change:

```bash
cd desktop-app/src-tauri
inkscape "$(pwd)/icon-source.svg" --export-type=png \
  --export-filename="$(pwd)/icon-source.png" -w 1024 -h 1024
cd ..
npm run tauri icon src-tauri/icon-source.png
rm src-tauri/icon-source.png
# tauri icon also emits android/ and ios/ sets — remove them, this is a desktop-only app
rm -rf src-tauri/icons/android src-tauri/icons/ios
```

`tauri.conf.json` references the icon set explicitly under `bundle.icon`. **This key is
required**: without it the `.deb` ships no icons and the AppImage build aborts with
`couldn't find a square icon to use as AppImage icon`.

## Scope

D1 covers the local build only. Automated CI builds (D2), signing and verification (D3+),
reproducibility (D4), and macOS/Windows (D5+) are tracked in the
[executable delivery plan](./executable-delivery-plan.md).
