# Local Dev Smoke Test Guide (APB)

> **Who this guide is for:** anyone — even with zero prior experience — who wants to try the
> Alpen Multisig application **end-to-end on their own machine**, starting from a fresh copy of
> the source code. You do **not** need to understand Bitcoin internals, Rust, or the Strata
> protocol to follow it. Every step is a copy‑paste command or a click, and after each one we
> tell you exactly what success looks like.
>
> **What you will achieve:** a fully working local test network (a private Bitcoin "regtest"
> chain plus every supporting service), the desktop app running in development mode, and one
> complete governance action — creating a proposal, signing it with two signers, and
> broadcasting it on your local chain.
>
> **How long it takes:** about 30–60 minutes the first time (most of it is the computer building
> things while you wait), and a few minutes on later runs.

This is the **single detailed walkthrough** for trying the app locally. For other topics, see:

- Installing a packaged release (not source): [Setup Guide](./setup-guide.md)
- How the system is put together: [Architecture Overview](./architecture-overview.md)
- Backend endpoints and authentication: [API Reference](./api-reference.md)
- Supported hardware signing devices: [Hardware Wallet Compatibility Matrix](./hardware-wallet-matrix.md)

---

## Table of contents

1. [Before you start: what you are about to run](#1-before-you-start-what-you-are-about-to-run)
2. [Prerequisites (install these once)](#2-prerequisites-install-these-once)
3. [Get the source code](#3-get-the-source-code)
4. [Bring up the local services (the "stack")](#4-bring-up-the-local-services-the-stack)
5. [Confirm every service is healthy](#5-confirm-every-service-is-healthy)
6. [Configure and start the desktop app](#6-configure-and-start-the-desktop-app)
7. [Happy‑path governance smoke test](#7-happy-path-governance-smoke-test)
8. [Shutting everything down](#8-shutting-everything-down)
9. [Troubleshooting (read this when something breaks)](#9-troubleshooting-read-this-when-something-breaks)
10. [Quick reference card](#10-quick-reference-card)

---

## 1. Before you start: what you are about to run

The app does not work alone — it talks to a small set of background **services**. The helper
script in this repository starts all of them for you inside **Docker** (a tool that runs
software in isolated "containers" so you don't have to install each piece by hand).

| Service | What it does (in plain words) | Runs on your machine at |
|---|---|---|
| Bitcoin (regtest) | A private, throwaway Bitcoin network only you can see. Lets you mine blocks and send coins freely. | `localhost:18443` |
| electrs | Lets the app quickly look up wallet balances on the chain. | `localhost:60401` |
| ASM | The Strata "Anchor State Machine" that processes governance actions. | `localhost:8080` |
| Orchestrator | The backend that coordinates proposals and collects signatures. | `localhost:3000` |
| Regtest Dev API | A convenience helper to mine blocks and hand out free test coins. | `localhost:3001` |
| PostgreSQL | The database the backend uses. | `localhost:5432` |

> **"regtest" in one sentence:** it is a fake Bitcoin network meant for testing — coins have no
> value, and you create blocks on demand by "mining" them with a single command.

You will also run the **desktop app** itself in *development mode*, which means it runs straight
from the source code (no installer needed) and reloads as code changes.

---

## 2. Prerequisites (install these once)

You need a **Linux** or **macOS** machine with a graphical desktop (the app opens a window, so a
headless/SSH‑only server will not work for the final UI steps).

Install the tools below. After each install, run the **"Verify"** command — if it prints a
version number, you are good.

### 2.1 Docker and Docker Compose v2

Runs all the background services.

- Install: follow the official guide for your OS at <https://docs.docker.com/get-docker/>.
- On Linux, make sure your user can run Docker without `sudo` (see Docker's "post‑install" steps).

**Verify:**

```bash
docker --version
docker compose version
```

Expected: two version lines, e.g. `Docker version 27.x` and `Docker Compose version v2.x`.
Also make sure the Docker engine is actually **running** (on macOS, open Docker Desktop):

```bash
docker ps
```

Expected: a table header (it can be empty) and **no** error like `Cannot connect to the Docker daemon`.

### 2.2 Git

Used to download the source code.

**Verify:**

```bash
git --version
```

Expected: `git version 2.x`.

> You also need **access to the private `asm` component** that ships as a Git "submodule". This
> is normally granted through your Alpen Labs GitHub account over SSH. If you can open
> <https://github.com/alpenlabs/asm> while logged in, you have access. If you are not sure, set
> up an SSH key now: <https://docs.github.com/en/authentication/connecting-to-github-with-ssh>.

### 2.3 Rust toolchain

Used to build the backend and the desktop app's native side.

- Install via rustup: <https://rustup.rs> (accept the defaults).
- This project pins a specific Rust version in `rust-toolchain.toml`. You do **not** need to pick
  it manually — the first build will download and use it automatically.

**Verify:**

```bash
rustc --version
cargo --version
```

Expected: both print a version. (They may show a different version than the project pin; that's
fine — the pinned one is fetched on first build.)

### 2.4 Node.js 20

Used to build the desktop app's user interface.

- The project expects **Node 20** (see the `.nvmrc` file). The easiest way is
  [nvm](https://github.com/nvm-sh/nvm):

```bash
nvm install 20
nvm use 20
```

**Verify:**

```bash
node --version   # should print v20.x
npm --version
```

### 2.5 Command‑line helpers: `curl` and `jq`

The startup script uses these to check health and hand out test coins.

**Verify:**

```bash
curl --version
jq --version
```

If `jq` is missing: `sudo apt install jq` (Linux) or `brew install jq` (macOS).

### 2.6 Tauri system libraries (the desktop app's GUI engine)

The desktop window is built with **Tauri**, which needs a few system libraries.

- **Linux (Debian/Ubuntu):**

  ```bash
  sudo apt update
  sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  ```

- **macOS:** install the Xcode Command Line Tools:

  ```bash
  xcode-select --install
  ```

> Full, always‑current Tauri prerequisites for every OS:
> <https://tauri.app/start/prerequisites/>.

**Checkpoint — you are ready when:** every "Verify" command above printed a version and
`docker ps` did not error.

---

## 3. Get the source code

Use the **`main`** branch — that is where released, deliverable code lives. The `asm` component
is a **submodule**, so it must be downloaded together with the main code; the
`--recurse-submodules` flag does that in one go.

```bash
git clone --branch main --recurse-submodules https://github.com/wakeuplabs-io/alpen-multisig.git
cd alpen-multisig
```

**If you already cloned without submodules**, fetch them now:

```bash
git submodule update --init asm
```

**Verify the submodule is present** (this file must exist):

```bash
test -f asm/Cargo.toml && echo "OK: asm submodule is present" || echo "MISSING: run git submodule update --init asm"
```

Expected: `OK: asm submodule is present`.

> From here on, **run every command from the repository's top folder** (the `alpen-multisig`
> directory you just entered) unless a step says otherwise.

---

## 4. Bring up the local services (the "stack")

One script builds and starts every background service for you. The **first run downloads and
builds Docker images, so it can take 10–20 minutes.** This is normal — let it finish.

```bash
./scripts/local-stack.sh
```

**What you should see:** the script prints progress sections such as
`=== docker compose build ===`, `=== docker compose up (detached) ===`,
`=== waiting for services to stabilize ===`, and finally `=== stack is up ===` followed by a
status table where services show `✅ healthy` or `✅ running`.

When it finishes, the services keep running in the background (detached), so you get your
terminal back.

> **Tip — start fresh anytime:** if a previous attempt left things in a weird state, run a clean
> start, which wipes the throwaway chain data and rebuilds from zero:
>
> ```bash
> ./scripts/local-stack.sh --clean
> ```

---

## 5. Confirm every service is healthy

**Do not open the app yet.** First make sure every service is up. Run:

```bash
./scripts/local-stack.sh --status
```

**What you should see:** a list ending with health markers. A healthy stack looks like:

```
  bitcoin              ✅ healthy   (:18443)
  electrs              ✅ healthy   (:60401)
  asm                  ✅ healthy   (:8080)
  postgres             ✅ healthy   (:5432)
  orchestrator         ✅ healthy   (:3000)
  regtest-dev-api      ✅ healthy   (:3001)
```

If any line shows `🔄 starting`, wait a minute and run the command again — ASM can take a little
while on the first boot.

### Optional: check each service yourself

These commands prove each service answers. Each should return data (not an error):

```bash
# Backend (orchestrator) health
curl http://localhost:3000/api/v1/health

# Regtest helper — mine one block (also confirms it works)
curl -X POST "http://localhost:3001/mine?count=1"

# ASM status
curl -X POST http://localhost:8080/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"strata_asm_getStatus","id":1}'

# Wallet indexer (electrs)
./scripts/smoke-electrs.sh   # expected: "OK: electrs is up and indexing."
```

**Checkpoint — you are ready for the app when:** `--status` shows all services healthy and the
orchestrator health check returns a response.

---

## 6. Configure and start the desktop app

### 6.1 Create the app's configuration file

The desktop app reads a small settings file named `desktop-app/.env`. Create it with the values
that match the local stack. Copy‑paste this whole block exactly:

```bash
cat > desktop-app/.env <<'EOF'
VITE_ORCHESTRATOR_BASE_URL=http://127.0.0.1:3000/api/v1
BITCOIN_NETWORK=regtest
BITCOIN_MAGIC_BYTES_HEX=414c504e
BITCOIN_RPC_URL=http://127.0.0.1:18443
BITCOIN_RPC_USER=user
BITCOIN_RPC_PASS=password
STRATA_ADMIN_STATE_RPC_URL=http://127.0.0.1:8080
EOF
echo "desktop-app/.env created"
```

> **Why these matter:** they point the app at your **local** backend and Bitcoin node and tell it
> to use the **regtest** network. The username/password (`user`/`password`) must match the local
> stack — if you change them, wallet balances will not load.

### 6.2 Install the UI dependencies

The first time only, download the front‑end packages:

```bash
cd desktop-app
npm install
```

Expected: it finishes with a summary like `added N packages` and no red `ERR!` lines.

### 6.3 Start the app in development mode

```bash
npm run tauri dev
```

**What you should see:** the terminal compiles the native side (first time can take several
minutes), then a **desktop window opens** showing the Alpen Multisig welcome / wallet‑connect
screen. Leave this terminal running — closing it closes the app.

**Checkpoint:** the app window is open and the stack from Step 5 is still healthy. You are ready
to run the smoke test.

---

## 7. Happy‑path governance smoke test

You will play **two signers** in turn, using two built‑in test seed phrases ("mnemonics") that
the local network already recognises as Strata Administrators. Threshold for this action is
**2 signatures**, so you will sign once as each signer and then broadcast.

> **Copy these exactly** (they differ only in the **last word**):
>
> - **Primary signer:**
>   `multiply toss magic exclude crawl obey garden black apart room village neglect`
> - **Co‑signer:**
>   `multiply toss magic exclude crawl obey garden black apart room village absent`
>
> These are **test‑only** phrases for the local fake network. Never use them on a real network or
> with real funds.

### Step 7.1 — Connect as the primary signer

1. On the wallet‑connect screen, choose the seed‑phrase option (labeled **"Palabras" / words**)
   and paste the **primary signer** phrase into the text box.
2. Confirm to connect with those words.
3. On **"Select authority"**, wait until **"Strata Administrator"** shows an **"Available"**
   badge — the app is checking your signer is recognised on the local chain. This can take a few
   seconds.
4. Select **Strata Administrator**, then click **Continue**.
5. On **"Authenticate session"**, click the authenticate button.

**What you should see:** the app lands on the **Proposals** screen (a dashboard listing
proposals, empty on a fresh start).

> If "Strata Administrator" never becomes "Available", your ASM service likely isn't fully up —
> recheck Step 5, mine a block (`./scripts/local-stack.sh --mine`), and try again.

### Step 7.2 — Create a proposal (as the primary signer)

1. From the Proposals dashboard, click **Create proposal**.
2. Choose the **Signer update** action card.
3. Give it a **title** (anything, e.g. `Smoke test add signer`).
4. In the new‑signer field, paste this test public key and click **Add**:

   ```
   03dd6d7dbd51e832af4c8eba8a7bf08ae616054b3e2e2e0823a8167c4def1e427c
   ```

   You should see the key appear in the "added signers" list.
5. Click **Preview** to reach the **Review** screen.
6. Click **Sign** (sign & submit).

**What you should see:** a **signature success** confirmation. The proposal now exists with
**one of two** required signatures. Back on the Proposals dashboard it appears as pending /
awaiting more signatures.

### Step 7.3 — Co‑sign with the second signer

The same person can act as the co‑signer by reconnecting with the other phrase:

1. **Disconnect** the current session (the disconnect control is in the screen header).
2. Repeat **Step 7.1**, but paste the **co‑signer** phrase this time.
3. Open the **pending proposal** you just created.
4. Click **Sign**.

**What you should see:** the proposal now has **2 of 2** signatures and moves to a **"Quorum
reached"** state. A **Broadcast** action becomes available on that proposal.

### Step 7.4 — Broadcast the action onto regtest

Broadcasting happens through a **commit → reveal** flow, and it needs a tiny amount of test coins
in the app's "Admin Wallet" to pay fees. You will fund it, then broadcast.

1. On the quorum‑reached proposal, click **Broadcast**. The **"Broadcast proposal"** screen
   opens.
2. The screen shows an **Admin Wallet funding address** (a `bcrt1...` address). **Copy it.**
3. In a **separate terminal** (keep the app running), give that address some test coins using the
   faucet helper — replace `<ADDRESS>` with the one you copied:

   ```bash
   ./scripts/local-stack.sh --fund <ADDRESS> 1
   ```

   Expected: it prints a `TXID:` and `Block:` line and `Done.`
4. Back in the app, open the **wallet panel** and click **Sync** (the same "refresh balance"
   action a real signer would use after receiving coins). Wait for the sync to finish, then close
   the panel.
5. The **Confirm & Broadcast** button becomes enabled once the synced balance shows up. Click it.

**What you should see:** a **phase progress** view with steps **Commit → Reveal → Enactment**.
The first heading reads something like *"Broadcasting…"* and then
*"Submitted — awaiting confirmation…"*.

6. The commit/reveal steps each wait for a block to confirm. On regtest, **you** produce blocks.
   In your separate terminal, mine a few blocks to move things along (repeat as the steps
   advance):

   ```bash
   ./scripts/local-stack.sh --mine 1
   ```

   Mine one, watch the screen advance, mine again if it's still waiting. A handful of blocks is
   plenty.

**✅ What "done" looks like:** the phase progress reaches a completion heading such as
**"Reveal confirmed"** / **"Proposal enacted"**, and a **done banner** appears. That means your
governance action was committed and revealed on your local regtest chain — a full end‑to‑end
success.

> **No device prompt appears** in this flow because you signed with seed phrases, not a hardware
> wallet. That is expected. To try a hardware‑wallet signer instead, see the
> [Hardware Wallet Compatibility Matrix](./hardware-wallet-matrix.md).

---

## 8. Shutting everything down

1. In the desktop‑app terminal, press **Ctrl + C** to close the app.
2. Stop the background services:

   ```bash
   ./scripts/local-stack.sh --stop
   ```

3. To also delete the throwaway chain/database data (full reset for next time):

   ```bash
   ./scripts/local-stack.sh --clean
   ```

---

## 9. Troubleshooting (read this when something breaks)

| Symptom | Likely cause | Fix |
|---|---|---|
| `Cannot connect to the Docker daemon` | Docker engine isn't running | Start Docker (open Docker Desktop on macOS; `sudo systemctl start docker` on Linux). Re‑check with `docker ps`. |
| `asm submodule not populated` / `MISSING: run git submodule update` | Submodule not downloaded | `git submodule update --init asm`. If it fails with a permission error, set up GitHub SSH access (see [2.2](#22-git)). |
| Startup fails with **"port is already allocated"** or **"address already in use"** | Another program (or an old run) is using a needed port (`18443`, `60401`, `8080`, `3000`, `3001`, `5432`) | Find and stop it: `lsof -i :3000` (swap in the port), then stop that process — or `./scripts/local-stack.sh --stop` to clear a previous run. |
| `--status` shows a service as `🔄 starting` or `❌` | Service still booting, or it crashed | Wait a minute and re‑run `--status`. If still failing, view logs: `docker compose -f staging/docker-compose.local.yml logs <service>` (e.g. `asm`). |
| "Strata Administrator" never turns **Available** at login | ASM not fully synced, or chain has no blocks yet | Confirm ASM is healthy (Step 5), then `./scripts/local-stack.sh --mine 1` and retry the login. |
| Wallet balance stays **0** after funding | Sync lag, or wrong RPC credentials | Click **Sync** again and wait; mine a block (`--mine 1`) so the funding transaction confirms; verify `desktop-app/.env` still has `BITCOIN_RPC_USER=user` and `BITCOIN_RPC_PASS=password`. |
| **Confirm & Broadcast** stays disabled | Admin Wallet balance not picked up yet | Make sure you funded the address from the screen, then **Sync** in the wallet panel and wait for it to finish. |
| Broadcast seems **stuck** on Commit or Reveal | Those steps wait for block confirmations | Mine blocks: `./scripts/local-stack.sh --mine 1` (repeat a few times). |
| App window never opens / native build errors on Linux | Missing Tauri system libraries | Re‑install the packages in [2.6](#26-tauri-system-libraries-the-desktop-apps-gui-engine). |
| App can't reach the backend | Wrong/missing config, or backend down | Confirm `desktop-app/.env` exists with `VITE_ORCHESTRATOR_BASE_URL=http://127.0.0.1:3000/api/v1`, and that `curl http://localhost:3000/api/v1/health` responds. |

**Still stuck?** Capture the relevant logs and open an issue at
<https://github.com/wakeuplabs-io/alpen-multisig/issues>.

---

## 10. Quick reference card

**Test seed phrases (regtest only):**

- Primary: `multiply toss magic exclude crawl obey garden black apart room village neglect`
- Co‑signer: `multiply toss magic exclude crawl obey garden black apart room village absent`

**Test signer public key (for the add‑signer proposal):**

```
03dd6d7dbd51e832af4c8eba8a7bf08ae616054b3e2e2e0823a8167c4def1e427c
```

**Everyday commands (run from the repository root):**

| Goal | Command |
|---|---|
| Start everything | `./scripts/local-stack.sh` |
| Clean start (wipe data) | `./scripts/local-stack.sh --clean` |
| Check health | `./scripts/local-stack.sh --status` |
| Mine N blocks | `./scripts/local-stack.sh --mine N` |
| Fund an address | `./scripts/local-stack.sh --fund <ADDRESS> 1` |
| Stop everything | `./scripts/local-stack.sh --stop` |
| Start the app | `cd desktop-app && npm run tauri dev` |

**Service ports:** Bitcoin `18443` · electrs `60401` · ASM `8080` · Orchestrator `3000` ·
Regtest Dev API `3001` · PostgreSQL `5432`.
