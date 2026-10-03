#!/bin/sh
# Run on inference03 with a task-owned scratch binary or the installed binary.
# No inference requests, user settings, or service changes.
set -eu
binary=${1:-/usr/local/bin/llm-visuals}
scratch=$(mktemp -d /tmp/llm-visuals-strata-flash-capture.XXXXXX)
socket=$scratch/tmux.sock
cleanup() { tmux -S "$socket" kill-server 2>/dev/null || true; }
trap cleanup EXIT
TERM=xterm-256color tmux -S "$socket" new-session -d -s verify -x 150 -y 46 \
  "env -u LLM_ENDPOINT -u OPENAI_BASE_URL TERM=xterm-256color XDG_CONFIG_HOME='$scratch/config' timeout 90 '$binary' --log-db off"
sleep 8
printf '\n=== initial default dashboard (150x46) ===\n'
tmux -S "$socket" capture-pane -p -t verify
tmux -S "$socket" send-keys -t verify v
sleep 1
printf '\n=== both models, comparison ===\n'
tmux -S "$socket" capture-pane -p -t verify
tmux -S "$socket" send-keys -t verify a
sleep 1
if ! tmux -S "$socket" capture-pane -p -t verify | head -n 2 | grep -q 'strata'; then
  tmux -S "$socket" send-keys -t verify Tab
  sleep 1
fi
printf '\n=== Flash-Next, all panels ===\n'
tmux -S "$socket" capture-pane -p -t verify
tmux -S "$socket" send-keys -t verify h
sleep 1
printf '\n=== Flash-Next, layers ===\n'
tmux -S "$socket" capture-pane -p -t verify
tmux -S "$socket" send-keys -t verify p
sleep 1
printf '\n=== Flash-Next, perf ===\n'
tmux -S "$socket" capture-pane -p -t verify
tmux -S "$socket" resize-window -t verify -x 100 -y 30
sleep 1
printf '\n=== Flash-Next, perf (100x30) ===\n'
tmux -S "$socket" capture-pane -p -t verify
tmux -S "$socket" resize-window -t verify -x 150 -y 46
sleep 1
tmux -S "$socket" send-keys -t verify Tab
sleep 1
tmux -S "$socket" send-keys -t verify a
sleep 1
printf '\n=== Qwen 27B, all panels ===\n'
tmux -S "$socket" capture-pane -p -t verify
tmux -S "$socket" send-keys -t verify q
