#!/bin/sh
set -eu
scratch=/tmp/llm-visuals-mixed-gpu-v2
socket=$scratch/tmux.sock
binary=${1:-/usr/local/bin/llm-visuals}
mode=${2:-user}
extra=${3:-}
mkdir -p "$scratch"
cleanup() { tmux -S "$socket" kill-server 2>/dev/null || true; }
trap cleanup EXIT
cleanup
TERM=xterm-256color tmux -S "$socket" new-session -d -s check -x "${CAPTURE_WIDTH:-200}" -y "${CAPTURE_HEIGHT:-60}" '/bin/bash --noprofile --norc'
if [ "$mode" = root ]; then prefix='sudo -n'; else prefix=''; fi
command="$prefix env -u LLM_ENDPOINT -u OPENAI_BASE_URL TERM=xterm-256color XDG_CONFIG_HOME='$scratch/config' timeout 45 '$binary' --log-db off $extra"
tmux -S "$socket" send-keys -t check -l "$command"
tmux -S "$socket" send-keys -t check Enter
sleep 6
echo '=== command / process / initial capture ==='
echo "$command"
ps -eo pid,ppid,args | grep -E 'llm-visuals|llama-server|/opt/strata/build/strata|serve.server' | grep -v grep
# Plain text keeps committed evidence easy to inspect.
tmux -S "$socket" capture-pane -p -t check
tmux -S "$socket" send-keys -t check v
sleep 2
echo '=== model comparison ==='
tmux -S "$socket" capture-pane -p -t check
tmux -S "$socket" send-keys -t check Tab a
sleep 2
echo '=== selected llama.cpp dashboard ==='
tmux -S "$socket" capture-pane -p -t check
