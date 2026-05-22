#!/usr/bin/env bash
# Start or stop Bitcoin Core (regtest) with RPC/ZMQ settings aligned to asm-runner's config.toml.
#
# Usage:
#   ./scripts/bitcoind-asm-runner.sh start|stop|restart|status
#
# Environment:
#   ASM_RUNNER_CONFIG   Path to config.toml (default: scripts/asm-config.toml)
#   BITCOIND_DATADIR    Bitcoin datadir for this node (default: ~/.bitcoin/asm-runner-regtest)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG="${ASM_RUNNER_CONFIG:-$ROOT/scripts/asm-config.toml}"
DATADIR="${BITCOIND_DATADIR:-$HOME/.bitcoin/asm-runner-regtest}"

RPC_HOST="127.0.0.1"
RPC_PORT="18443"
RPC_USER="user"
RPC_PASSWORD="password"
ZMQ_RAWBLOCK="tcp://127.0.0.1:28332"

load_bitcoin_section() {
  [[ -f "$CONFIG" ]] || return 0
  local in_section=0
  local line
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    case "$line" in
      "[bitcoin]") in_section=1 ;;
      \[*\])
        if [[ $in_section -eq 1 ]]; then
          in_section=0
        fi
        ;;
      *)
        if [[ $in_section -ne 1 ]]; then
          continue
        fi
        if [[ "$line" =~ ^rpc_url[[:space:]]*=[[:space:]]*\"http://([^:/]+):([0-9]+)\" ]]; then
          RPC_HOST="${BASH_REMATCH[1]}"
          RPC_PORT="${BASH_REMATCH[2]}"
        elif [[ "$line" =~ ^rpc_user[[:space:]]*=[[:space:]]*\"([^\"]*)\" ]]; then
          RPC_USER="${BASH_REMATCH[1]}"
        elif [[ "$line" =~ ^rpc_password[[:space:]]*=[[:space:]]*\"([^\"]*)\" ]]; then
          RPC_PASSWORD="${BASH_REMATCH[1]}"
        elif [[ "$line" =~ ^rawblock_connection_string[[:space:]]*=[[:space:]]*\"([^\"]*)\" ]]; then
          ZMQ_RAWBLOCK="${BASH_REMATCH[1]}"
        fi
        ;;
    esac
  done <"$CONFIG"
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: '$1' not found in PATH" >&2
    exit 1
  }
}

cli_args() {
  echo -rpcconnect="$RPC_HOST" -rpcport="$RPC_PORT" -rpcuser="$RPC_USER" -rpcpassword="$RPC_PASSWORD"
}

cmd_start() {
  require_cmd bitcoind
  load_bitcoin_section
  mkdir -p "$DATADIR"
  # shellcheck disable=SC2046
  if bitcoin-cli -regtest -datadir="$DATADIR" $(cli_args) getblockchaininfo >/dev/null 2>&1; then
    echo "bitcoind already running (datadir=$DATADIR)"
    exit 0
  fi
  bitcoind -regtest -datadir="$DATADIR" -daemon \
    -server=1 \
    -txindex=1 \
    -rpcbind="$RPC_HOST" \
    -rpcallowip="$RPC_HOST" \
    -rpcuser="$RPC_USER" \
    -rpcpassword="$RPC_PASSWORD" \
    -rpcport="$RPC_PORT" \
    -zmqpubrawblock="$ZMQ_RAWBLOCK"
  echo "bitcoind started (datadir=$DATADIR, rpc=$RPC_HOST:$RPC_PORT, zmq=$ZMQ_RAWBLOCK)"
}

cmd_stop() {
  require_cmd bitcoin-cli
  load_bitcoin_section
  # shellcheck disable=SC2046
  if ! bitcoin-cli -regtest -datadir="$DATADIR" $(cli_args) getblockchaininfo >/dev/null 2>&1; then
    echo "bitcoind not running (datadir=$DATADIR)"
    exit 0
  fi
  # shellcheck disable=SC2046
  bitcoin-cli -regtest -datadir="$DATADIR" $(cli_args) stop
  echo "bitcoind stop requested (datadir=$DATADIR)"
}

cmd_status() {
  require_cmd bitcoin-cli
  load_bitcoin_section
  # shellcheck disable=SC2046
  if bitcoin-cli -regtest -datadir="$DATADIR" $(cli_args) getblockchaininfo >/dev/null 2>&1; then
    echo "bitcoind: running (datadir=$DATADIR)"
    # shellcheck disable=SC2046
    bitcoin-cli -regtest -datadir="$DATADIR" $(cli_args) getblockchaininfo | head -n 20
  else
    echo "bitcoind: not running (datadir=$DATADIR)"
    exit 1
  fi
}

cmd_restart() {
  cmd_stop || true
  sleep 1
  cmd_start
}

usage() {
  echo "usage: $0 {start|stop|restart|status}" >&2
  echo "" >&2
  echo "  ASM_RUNNER_CONFIG=${ASM_RUNNER_CONFIG:-$ROOT/asm/config.toml}" >&2
  echo "  BITCOIND_DATADIR=${BITCOIND_DATADIR:-$HOME/.bitcoin/asm-runner-regtest}" >&2
  exit 1
}

main() {
  case "${1:-}" in
    start) cmd_start ;;
    stop) cmd_stop ;;
    restart) cmd_restart ;;
    status) cmd_status ;;
    *) usage ;;
  esac
}

main "$@"
