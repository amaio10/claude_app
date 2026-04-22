#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

if [ ! -f .env ]; then
	echo "error: .env missing — copy .env.example and fill GROQ_API_KEY"
	exit 1
fi

if [ -f "$HOME/.nvm/nvm.sh" ]; then
	# shellcheck disable=SC1091
	. "$HOME/.nvm/nvm.sh" >/dev/null
fi

echo "→ starting backend (cargo run) on :7777"
(cd backend && RUST_LOG=info cargo run) &
BACKEND_PID=$!

cleanup() {
	echo "→ stopping"
	kill "$BACKEND_PID" 2>/dev/null || true
	kill "$FRONTEND_PID" 2>/dev/null || true
	wait 2>/dev/null || true
}
trap cleanup INT TERM EXIT

sleep 2
echo "→ starting frontend (vite) on :5173"
(cd frontend && pnpm run dev --host 127.0.0.1) &
FRONTEND_PID=$!

echo
echo "  open → http://127.0.0.1:5173"
echo "  backend logs are above; Ctrl+C to stop"
echo

wait
