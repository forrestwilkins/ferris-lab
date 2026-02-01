#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_PATH="${BIN_PATH:-"$ROOT_DIR/target/debug/ferris-lab"}"

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
cargo build

pids=()
cleanup() {
  for pid in "${pids[@]:-}"; do
    kill "$pid" 2>/dev/null || true
  done
  wait 2>/dev/null || true
}
trap cleanup INT TERM EXIT

for env_block in "$@"; do
  if command -v stdbuf >/dev/null 2>&1; then
    stdbuf -oL -eL env $env_block "$BIN_PATH" &
  else
    env $env_block "$BIN_PATH" &
  fi
  pids+=("$!")
done

wait
