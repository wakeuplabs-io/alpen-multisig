# Local Dev Smoke Test Guide

> **Who this is for:** anyone — even with zero experience — who wants to try the Alpen Multisig
> app **end-to-end on their own machine**, starting from a fresh copy of the source code. You do
> **not** need to know Bitcoin, Rust, or the Strata protocol. Every step is a copy‑paste command
> or a click, and after each one we tell you what success looks like.
>
> **What you'll do:** start a private throwaway test network on your computer, open the app, and
> complete one full governance action — create a proposal, sign it with two signers, and
> broadcast it.
>
> **Time:** ~30–60 min the first time (mostly your computer building things while you wait), a
> few minutes after that.

**Good news:** there is almost nothing to configure. The helper script starts every service for
you, and the app already points at that local network by default. You mostly run two commands and
then click through the app.

For other topics see: [Setup Guide](./setup-guide.md) (install a packaged release),
[Architecture Overview](./architecture-overview.md), [API Reference](./api-reference.md), and the
[Hardware Wallet Compatibility Matrix](./hardware-wallet-matrix.md). This document is the single
detailed walkthrough for trying the app locally.

---

## Table of contents

1. [Install the tools (once)](#1-install-the-tools-once)
2. [Get the source code](#2-get-the-source-code)
3. [Start the local network](#3-start-the-local-network)
4. [Check everything is healthy](#4-check-everything-is-healthy)
5. [Start the app](#5-start-the-app)
6. [Run the smoke test](#6-run-the-smoke-test)
7. [Shut everything down](#7-shut-everything-down)
8. [Troubleshooting](#8-troubleshooting)
9. [Quick reference](#9-quick-reference)

---

## 1. Install the tools (once)

You need a **Linux** or **macOS** machine with a graphical desktop (the app opens a window, so a
headless/SSH‑only server won't work).

Install each tool below, then run its **Verify** line — if it prints a version, you're good.

| Tool | What it's for | Install | Verify |
|---|---|---|---|
| **Docker** + Compose v2 | Runs all the background services | <https://docs.docker.com/get-docker/> | `docker compose version` and `docker ps` (must not error) |
| **Git** | Downloads the code | your OS package manager | `git --version` |
| **Rust** (rustup) | Builds the app's native side | <https://rustup.rs> | `cargo --version` |
| **Node.js 20** | Builds the app's interface | `nvm install 20 && nvm use 20` | `node --version` (v20.x) |
| **curl** + **jq** | Used by the helper script | `sudo apt install jq` / `brew install jq` | `jq --version` |

**Two extra notes:**

- **Make sure Docker is actually running** before you start (on macOS, open Docker Desktop).
  `docker ps` should print a table header with no error.
- **You need access to the private `asm` submodule.** This comes with your Alpen Labs GitHub
  account over SSH — if you can open <https://github.com/alpenlabs/asm> while logged in, you're
  set. (Set up SSH if needed: <https://docs.github.com/en/authentication/connecting-to-github-with-ssh>.)

**Tauri system libraries** (the app's window engine):

- **Linux (Debian/Ubuntu):**

  ```bash
  sudo apt update && sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  ```

- **macOS:** `xcode-select --install`

> Always‑current Tauri prerequisites for every OS: <https://tauri.app/start/prerequisites/>.

---

## 2. Get the source code

Use the **`main`** branch — that's where released, deliverable code lives. The `--recurse-submodules`
flag also downloads the `asm` component in one go.

```bash
git clone --branch main --recurse-submodules https://github.com/wakeuplabs-io/alpen-multisig.git
cd alpen-multisig
```

**Already cloned without submodules?** Run this once:

```bash
git submodule update --init asm
```

**Check it worked:**

```bash
test -f asm/Cargo.toml && echo "OK" || echo "MISSING — run: git submodule update --init asm"
```

Expected: `OK`.

> From here on, run every command from this `alpen-multisig` folder.

---

## 3. Start the local network

One script builds and starts everything (a private Bitcoin test network plus all supporting
services). **The first run can take 10–20 minutes while it downloads and builds — that's normal.**

```bash
./scripts/local-stack.sh
```

**What you should see:** progress sections ending with `=== stack is up ===` and a status table
where services show `✅ healthy` or `✅ running`. When it finishes, the services keep running in
the background and you get your terminal back.

> **Want a clean slate later?** `./scripts/local-stack.sh --clean` wipes the throwaway data and
> starts fresh.

---

## 4. Check everything is healthy

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

## 5. Start the app

No configuration needed: the app already points at the local network you just started.

```bash
cd desktop-app
npm install        # first time only
npm run tauri dev
```

**What you should see:** it compiles (first time can take a few minutes), then a **desktop window
opens** on the welcome / connect screen. Leave this terminal running — closing it closes the app.

> If the app ever can't reach the network, open **Settings → Node** and confirm the connection
> mode is **Local** (the default). You shouldn't need to change anything.

---

## 6. Run the smoke test

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

## 7. Shut everything down

1. In the app terminal, press **Ctrl + C** to close the app.
2. Stop the services: `./scripts/local-stack.sh --stop`
3. (Optional) full reset, wiping test data: `./scripts/local-stack.sh --clean`

---

## 8. Troubleshooting

| Symptom | Fix |
|---|---|
| `Cannot connect to the Docker daemon` | Start Docker (Docker Desktop on macOS; `sudo systemctl start docker` on Linux), then re‑check with `docker ps`. |
| `asm submodule not populated` / `MISSING` | Run `git submodule update --init asm`. If it's a permission error, set up GitHub SSH access. |
| **"port already in use"** on startup | Something else uses a needed port (`18443`, `60401`, `8080`, `3000`, `3001`, `5432`). Find it with `lsof -i :3000` (swap the port) and stop it — or clear a previous run with `./scripts/local-stack.sh --stop`. |
| A service shows `🔄 starting` or `❌` | Wait a minute and re‑run `--status`. Still failing? View logs: `docker compose -f staging/docker-compose.local.yml logs <service>` (e.g. `asm`). |
| **Strata Administrator** never turns **Available** at login | The network needs a block: `./scripts/local-stack.sh --mine 1`, then retry the login. |
| Wallet balance stays **0** after funding | Mine a block so the funding confirms (`--mine 1`), then click **Sync** again and wait. |
| **Confirm & Broadcast** stays disabled | Make sure you funded the address shown on screen, then **Sync** in the wallet panel and wait for it to finish. |
| Broadcast seems **stuck** on Commit or Reveal | Those steps wait for confirmations — mine more blocks: `./scripts/local-stack.sh --mine 1` (repeat). |
| App window won't open / native build errors (Linux) | Re‑install the Tauri system libraries from [Step 1](#1-install-the-tools-once). |
| App can't reach the network | Open **Settings → Node** and confirm the mode is **Local**. Re‑check the stack with `./scripts/local-stack.sh --status`. |

**Still stuck?** Open an issue at <https://github.com/wakeuplabs-io/alpen-multisig/issues>.

---

## 9. Quick reference

**Test seed phrases (local network only):**

- Signer 1: `multiply toss magic exclude crawl obey garden black apart room village neglect`
- Signer 2: `multiply toss magic exclude crawl obey garden black apart room village absent`

**Test public key (for the proposal):**

```
03dd6d7dbd51e832af4c8eba8a7bf08ae616054b3e2e2e0823a8167c4def1e427c
```

**Commands (run from the repository root):**

| Goal | Command |
|---|---|
| Start the network | `./scripts/local-stack.sh` |
| Check health | `./scripts/local-stack.sh --status` |
| Mine 1 block | `./scripts/local-stack.sh --mine 1` |
| Fund an address | `./scripts/local-stack.sh --fund <ADDRESS> 1` |
| Stop everything | `./scripts/local-stack.sh --stop` |
| Clean reset | `./scripts/local-stack.sh --clean` |
| Start the app | `cd desktop-app && npm run tauri dev` |
