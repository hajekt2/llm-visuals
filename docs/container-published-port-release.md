# Container published port: v0.10.0-hajek.6

## Failure and cause

On inference03 after the Strata 0.2 upgrade, the dashboard showed the model name
but "Metrics unavailable - waiting for /metrics", and the history database
received two sample rows per second: one real, one all zeros.

Strata 0.2 runs in a Docker bridge network. Its process listens on
`0.0.0.0:8080` inside the container and `docker-proxy` publishes 8098 on
loopback and on the LAN address. `resolve_container_endpoint` compared the
server's network namespace with ours by reading `/proc/<pid>/ns/net`. That link
is only readable for a process we may ptrace, so for a container whose process
is owned by root it returns EPERM for an ordinary user. The old code read that
as "same namespace as us" and kept the command-line port, so the dashboard
polled `127.0.0.1:8080`, where nothing answers.

Two models then survived discovery: the pinned endpoint model on 8098 with real
metrics, and the process model on 8080 with none. The merge in `discover()`
deduplicates by port, so 8098 and 8080 never merged and both were logged.

## Fix

- A namespace link we cannot read no longer implies a shared namespace. The
  published host port is looked up instead.
- `docker_proxy_args` collects the proxies in our namespace, and
  `pick_published_endpoint` matches them on `-container-port`, requires `-proto
  tcp`, and prefers a loopback `-host-ip` because Docker starts one proxy per
  published address.
- `published_endpoint` first checks `/proc/net/tcp` and `/proc/net/tcp6`
  (`parse_listen_ports`, LISTEN state 0A). If the command-line port already
  listens where we are, nothing is rewritten, so a host-network engine is never
  redirected to another container that happens to publish the same internal port.
- `resolve_container_endpoint` now keeps the IP-anchored match when the
  namespace is readable, and falls back to the port-only match when it is not.

## Verification

- `cargo fmt --check`, `cargo test --locked` (184 passed, 2 ignored), and
  `cargo build --release --locked` pass. `cargo clippy --all-targets` reports no
  new warning from this change.
- Reproduced on inference03 with the pinned endpoint removed from the saved
  settings, so only `api-key-file` is configured: the pre-release binary attached
  to `qwen3.8-flash-next-iq3_s`, `strata pid 2790271`, `vision CPU`, with the
  STRATA panel live (`version 0.1.46`, request history, expert hit, conversation
  cache) and zero "Metrics unavailable" frames.
- The header lists one model, not `model 1/2`, which is the merge working: one
  watched model means one sample row per tick in the history database.
- Still unknown on this host: layer and head counts show `0L × 0H`. The engine
  reports no layer placement, and reading the GGUF header of the container's
  model path needs `/proc/<pid>/root`, which is also ptrace-gated. The shards
  themselves are world-readable on the host; resolving that is separate work.

## Release

- Tag: `v0.10.0-hajek.6`, published as a prerelease by the `release` workflow.
- Asset used by the homelab:
  `llm-visuals-0.10.0-hajek.6-linux-x86_64.tar.gz`
- Consumers pin version and SHA-256; see `inventory/host_vars/inference03.yml`
  in homelab-iac and `docs/AI_INFERENCE_VM.md`.
