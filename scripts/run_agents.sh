#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_PATH="${BIN_PATH:-"$ROOT_DIR/target/debug/ferris-lab"}"
MUX_PATH="${MUX_PATH:-"$ROOT_DIR/target/debug/log_mux"}"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/run_agents.sh "AGENT_ID=agent-1 AGENT_PORT=8080 PEER_ADDRESSES=ws://localhost:8081/ws" \
                          "AGENT_ID=agent-2 AGENT_PORT=8081 PEER_ADDRESSES=ws://localhost:8080/ws"

Notes:
  - Builds once, then runs all agents in parallel.
  - Set BIN_PATH to use a different binary (e.g. release build).
EOF
}

if [[ $# -lt 1 ]]; then
  usage
  exit 1
fi

cd "$ROOT_DIR"
cargo build --bin ferris-lab --bin log_mux

pids=()
fifo_dir=""
use_process_group=false
cleanup() {
  if [[ -n "${fifo_dir}" && -d "${fifo_dir}" ]]; then
    rm -rf "${fifo_dir}" || true
  fi
  for pid in "${pids[@]:-}"; do
    if $use_process_group; then
      kill -TERM "-$pid" 2>/dev/null || true
    else
      kill "$pid" 2>/dev/null || true
    fi
  done
  wait 2>/dev/null || true
}
trap cleanup INT TERM EXIT

fifo_dir="$(mktemp -d)"
declare -a mux_args

for env_block in "$@"; do
  agent_id="agent"
  for kv in $env_block; do
    case "$kv" in
      AGENT_ID=*) agent_id="${kv#AGENT_ID=}" ;;
    esac
  done

  fifo_path="${fifo_dir}/${agent_id}.fifo"
  mkfifo "$fifo_path"
  mux_args+=("${agent_id}=${fifo_path}")

  if command -v setsid >/dev/null 2>&1; then
    use_process_group=true
    setsid env $env_block "$BIN_PATH" >"$fifo_path" 2>&1 &
  else
    env $env_block "$BIN_PATH" >"$fifo_path" 2>&1 &
  fi
  pids+=("$!")
done

if [[ ! -x "$MUX_PATH" ]]; then
  echo "log_mux not found at $MUX_PATH" >&2
  exit 1
fi

"$MUX_PATH" "${mux_args[@]}"

wait
