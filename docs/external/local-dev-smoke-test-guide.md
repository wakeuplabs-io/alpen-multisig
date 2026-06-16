# Local Dev Smoke Test Guide

> **Who this is for:** anyone — even with zero experience — who wants to try the Alpen Multisig
> app **end-to-end on their own machine**. You do **not** need to know Bitcoin, Rust, or the
> Strata protocol, and **you don't build anything** — you download the ready‑made app from GitHub
> and run a few simple commands to start a private test network for it to talk to.
>
> **What you'll do:** start a private throwaway test network on your computer, install the app
> from the latest release, and complete one full governance action — create a proposal, sign it
> with two signers, and broadcast it.
>
> **Time:** ~20–40 min the first time (mostly your computer downloading and starting the test
> network), a few minutes after that.

**There is almost nothing to configure.** A helper script starts every service for you, the app
comes pre‑built from the releases page, and it already points at your local test network by
default. You start the network, open the app, and click through it.

For other topics see: [Setup Guide](./setup-guide.md), [Architecture Overview](./architecture-overview.md),
[API Reference](./api-reference.md), and the [Hardware Wallet Compatibility Matrix](./hardware-wallet-matrix.md).
This document is the single detailed walkthrough for trying the app locally.

---

## Table of contents

1. [Install the tools (once)](#1-install-the-tools-once)
2. [Start the local test network](#2-start-the-local-test-network)
3. [Check everything is healthy](#3-check-everything-is-healthy)
4. [Download and open the app](#4-download-and-open-the-app)
5. [Run the smoke test](#5-run-the-smoke-test)
6. [Shut everything down](#6-shut-everything-down)
7. [Troubleshooting](#7-troubleshooting)
8. [Quick reference](#8-quick-reference)

---

## 1. Install the tools (once)

You need a **Linux** or **macOS** machine with a graphical desktop. (On Windows, run the network
commands inside **WSL2**; the app itself has a native Windows installer.)

The test network runs in Docker — that's the only heavy tool. Install each item below, then run
its **Verify** line; if it prints a version, you're good.

| Tool | What it's for | Install | Verify |
|---|---|---|---|
| **Docker** + Compose v2 | Runs the local test network | <https://docs.docker.com/get-docker/> | `docker compose version` and `docker ps` (must not error) |
| **Git** | Downloads the network scripts | your OS package manager | `git --version` |
| **curl** + **jq** | Used by the helper script | `sudo apt install jq` / `brew install jq` | `jq --version` |

**Two notes:**

- **Make sure Docker is actually running** before you start (on macOS, open Docker Desktop).
  `docker ps` should print a table header with no error.
- **You need access to the private `asm` component.** It comes with your Alpen Labs GitHub account
  over SSH — if you can open <https://github.com/alpenlabs/asm> while logged in, you're set.
  (Set up SSH if needed: <https://docs.github.com/en/authentication/connecting-to-github-with-ssh>.)

> Note: you do **not** need Rust, Node, or any build tools — the app is downloaded ready to run.

---

## 2. Start the local test network

The app needs a private Bitcoin test network and a few supporting services to talk to. One script
starts them all. First, download the scripts (the `asm` component comes along via
`--recurse-submodules`):

```bash
git clone --branch main --recurse-submodules https://github.com/wakeuplabs-io/alpen-multisig.git
cd alpen-multisig
```

Then start everything. **The first run can take 10–20 minutes while it downloads and builds the
services — that's normal.**

```bash
./scripts/local-stack.sh
```

**What you should see:** progress sections ending with `=== stack is up ===` and a status table
where services show `✅ healthy` or `✅ running`. When it finishes, the services keep running in
the background and you get your terminal back.

> Run every command in this guide from this `alpen-multisig` folder.
> Want a clean slate later? `./scripts/local-stack.sh --clean` wipes the test data and starts fresh.

---

## 3. Check everything is healthy

Before opening the app, confirm every service is up:

```bash
./scripts/local-stack.sh --status
```

**What you should see** — every line healthy:

```
  bitcoin              ✅ healthy   (:18443)
  electrs              ✅ healthy   (:60401)
  asm                  ✅ healthy   (:8080)
  postgres             ✅ healthy   (:5432)
  orchestrator         ✅ healthy   (:3000)
  regtest-dev-api      ✅ healthy   (:3001)
```

If anything shows `🔄 starting`, wait a minute and run it again — the `asm` service can take a
little while on first boot.

---

## 4. Download and open the app

Go to the releases page and open the **latest** release:

**<https://github.com/wakeuplabs-io/alpen-multisig/releases/latest>**

Under **Assets**, download the file for your system, then open it:

| Your system | Download | Open it |
|---|---|---|
| **Linux (AppImage)** | `Alpen.Multisig_*_amd64.AppImage` | `chmod +x Alpen.Multisig_*_amd64.AppImage` then `./Alpen.Multisig_*_amd64.AppImage` |
| **Linux (Ubuntu/Debian)** | `Alpen.Multisig_*_amd64.deb` | `sudo apt install ./Alpen.Multisig_*_amd64.deb`, then launch **Alpen Multisig** from your apps menu |
| **Linux (Fedora/RHEL)** | `Alpen.Multisig-*.x86_64.rpm` | `sudo dnf install ./Alpen.Multisig-*.x86_64.rpm`, then launch **Alpen Multisig** |
| **macOS (Apple Silicon)** | `Alpen.Multisig_*_aarch64.dmg` | Open the `.dmg`, drag the app to **Applications**, then open it. First time: right‑click the app → **Open** to get past the security prompt. |
| **Windows** | `desktop-app-*-windows.exe` | Run the installer, then launch **Alpen Multisig**. |

**What you should see:** the app opens on the welcome / connect screen. It already points at the
local test network you started — no configuration needed.

> **Optional but recommended:** verify the download is authentic before running it. The release
> includes `SHA256SUMS` and a signature — see [Verifying a Release](./verifying-releases.md).
>
> If the app ever can't reach the network, open **Settings → Node** and confirm the connection
> mode is **Local** (the default).

---

## 5. Run the smoke test

You'll act as **two signers** in turn, using two built‑in test seed phrases that the local network
already recognises as administrators. This action needs **2 signatures**, so you sign once as each,
then broadcast.

> **Copy these exactly** — they differ only in the **last word**:
>
> - **Signer 1:** `multiply toss magic exclude crawl obey garden black apart room village neglect`
> - **Signer 2:** `multiply toss magic exclude crawl obey garden black apart room village absent`
>
> These are **test‑only** phrases for the local fake network. Never use them with real funds.

### Step 1 — Connect as Signer 1

1. On the connect screen, choose the seed‑phrase option (labeled **"Palabras" / words**) and paste
   **Signer 1**'s phrase.
2. Confirm to connect.
3. On **Select authority**, wait until **Strata Administrator** shows an **Available** badge (the
   app is checking your signer on‑chain — a few seconds). Select it and click **Continue**.
4. On **Authenticate session**, click the authenticate button.

**You should see:** the **Proposals** screen (empty on a fresh start).

### Step 2 — Create a proposal

1. Click **Create proposal**.
2. Choose the **Signer update** card.
3. Enter a **title** (e.g. `Smoke test`).
4. Paste this test public key and click **Add** — it should appear in the list:

   ```
   03dd6d7dbd51e832af4c8eba8a7bf08ae616054b3e2e2e0823a8167c4def1e427c
   ```

5. Click **Preview**, then on the review screen click **Sign**.

**You should see:** a signature success message. The proposal now has **1 of 2** signatures.

### Step 3 — Sign as Signer 2

1. **Disconnect** (control in the screen header).
2. Repeat **Step 1**, but paste **Signer 2**'s phrase.
3. Open the pending proposal and click **Sign**.

**You should see:** the proposal reaches **2 of 2** — **Quorum reached** — and a **Broadcast**
action appears.

### Step 4 — Broadcast it

Broadcasting needs a tiny amount of test coins for fees. You'll fund the wallet, then broadcast.

> **Prefer buttons over the terminal?** Open **`faucet-ui/index.html`** (double‑click it, or
> open the file in your browser) for a small page with **Send BTC** and **Mine Blocks** buttons —
> it does the same as the `--fund` and `--mine` commands below, and also lists the test seed
> phrases and their roles. Keep the local network running while you use it.

1. Click **Broadcast**. The **Broadcast proposal** screen opens and shows an **Admin Wallet
   funding address** (`bcrt1...`). **Copy it.**
2. In a **second terminal** (keep the app running), give that address test coins — replace
   `<ADDRESS>`:

   ```bash
   ./scripts/local-stack.sh --fund <ADDRESS> 1
   ```

   Expected: it prints `TXID:`, `Block:`, and `Done.`
3. Back in the app, open the **wallet panel**, click **Sync**, wait for it to finish, and close the
   panel.
4. The **Confirm & Broadcast** button becomes enabled — click it.
5. A progress view shows **Commit → Reveal → Enactment**. These steps wait for blocks, and on this
   test network **you** create blocks. In your second terminal, mine a few (repeat as the steps
   advance):

   ```bash
   ./scripts/local-stack.sh --mine 1
   ```

**✅ Done looks like:** the progress reaches a completion heading such as **Reveal confirmed** /
**Proposal enacted**, with a done banner. That's a full end‑to‑end success — your governance action
was committed and revealed on your local network.

> No device prompt appears because you signed with seed phrases, not a hardware wallet — that's
> expected. To try a hardware signer, see the
> [Hardware Wallet Compatibility Matrix](./hardware-wallet-matrix.md).

---

## 6. Shut everything down

1. Close the app window.
2. Stop the services: `./scripts/local-stack.sh --stop`
3. (Optional) full reset, wiping test data: `./scripts/local-stack.sh --clean`

---

## 7. Troubleshooting

| Symptom | Fix |
|---|---|
| `Cannot connect to the Docker daemon` | Start Docker (Docker Desktop on macOS; `sudo systemctl start docker` on Linux), then re‑check with `docker ps`. |
| `asm submodule not populated` / `MISSING` | Run `git submodule update --init asm`. If it's a permission error, set up GitHub SSH access. |
| **"port already in use"** on startup | Something else uses a needed port (`18443`, `60401`, `8080`, `3000`, `3001`, `5432`). Find it with `lsof -i :3000` (swap the port) and stop it — or clear a previous run with `./scripts/local-stack.sh --stop`. |
| A service shows `🔄 starting` or `❌` | Wait a minute and re‑run `--status`. Still failing? View logs: `docker compose -f staging/docker-compose.local.yml logs <service>` (e.g. `asm`). |
| AppImage won't start (Linux) | Make it executable: `chmod +x Alpen.Multisig_*_amd64.AppImage`, then run `./Alpen.Multisig_*_amd64.AppImage`. |
| macOS: *"app can't be opened"* | Right‑click the app → **Open** the first time to get past the security prompt. |
| **Strata Administrator** never turns **Available** at login | The network needs a block: `./scripts/local-stack.sh --mine 1`, then retry the login. |
| Wallet balance stays **0** after funding | Mine a block so the funding confirms (`--mine 1`), then click **Sync** again and wait. |
| **Confirm & Broadcast** stays disabled | Make sure you funded the address shown on screen, then **Sync** in the wallet panel and wait for it to finish. |
| Broadcast seems **stuck** on Commit or Reveal | Those steps wait for confirmations — mine more blocks: `./scripts/local-stack.sh --mine 1` (repeat). |
| App can't reach the network | Open **Settings → Node** and confirm the mode is **Local**. Re‑check the stack with `./scripts/local-stack.sh --status`. |
| App won't open (Linux) — missing system library error | The app needs the WebKit/GTK system libraries. Install them (see below) and try again. |

**App won't open on Linux (missing system libraries):** if the app fails to start with an error
about a missing `libwebkit2gtk` (or similar) library, install the system dependencies and retry:

```bash
sudo apt update && sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

On **macOS**, if the app won't open due to missing developer tools, run `xcode-select --install`.

**Still stuck?** Open an issue at <https://github.com/wakeuplabs-io/alpen-multisig/issues>.

---

## 8. Quick reference

**Get the app:** <https://github.com/wakeuplabs-io/alpen-multisig/releases/latest> (download the
asset for your system).

**Test seed phrases (local network only):**

- Signer 1: `multiply toss magic exclude crawl obey garden black apart room village neglect`
- Signer 2: `multiply toss magic exclude crawl obey garden black apart room village absent`

**Test public key (for the proposal):**

```
03dd6d7dbd51e832af4c8eba8a7bf08ae616054b3e2e2e0823a8167c4def1e427c
```

**Network commands (run from the repository root):**

| Goal | Command |
|---|---|
| Start the network | `./scripts/local-stack.sh` |
| Check health | `./scripts/local-stack.sh --status` |
| Mine 1 block | `./scripts/local-stack.sh --mine 1` |
| Fund an address | `./scripts/local-stack.sh --fund <ADDRESS> 1` |
| Stop everything | `./scripts/local-stack.sh --stop` |
| Clean reset | `./scripts/local-stack.sh --clean` |

> **Tip:** instead of the `--fund` and `--mine` commands, you can open **`faucet-ui/index.html`**
> in your browser for clickable **Send BTC** and **Mine Blocks** buttons (it also lists the test
> seed phrases and roles).
