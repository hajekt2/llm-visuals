# Strata Maya integration: v0.10.0-hajek.4

## Problem and fix

Inference03's installed `0.10.0-hajek.3` filled Strata panels for the
Flash-Next launcher, but Project Maya's launcher left the dashboard on
"Metrics unavailable - waiting for /metrics". Three facts combine:

- Maya's `serve/server.py` passes no `--host`; Maya issue #26 defers the bind
  address to the `--config` JSON (`"host": "172.16.0.73"`, port 8097). The
  config carries `model_name: glm-5.3-flash`.
- The Maya container is root-owned, so ordinary-user `haja` cannot read the
  config through `/proc/<pid>/root`, and `apply_strata_config` returned early.
- The container runs `network_mode: host` with a LAN-only bind, so the default
  dial address `127.0.0.1` refuses the connection, and no docker-proxy
  publishes the port.

Fixes, on top of the hajek.3 Strata adapter:

- When the launcher's mount namespace denies the config read, re-anchor the
  `--config` path through `/proc/<pid>/mountinfo` and read the host-side copy
  (the Ansible template publishes the same JSON). The bind address, port and
  model name now come from the config as intended.
- Maya's `strata-vision` encoder child folds into the launcher like the native
  engine child, but without renaming the server: its `--model` names only its
  own encoder shard. The server row gains the vision mark (CPU for the Maya
  vision profile) and the child's memory.
- `detect_vision` keeps a fold-set vision on Strata rows instead of clearing it.

No metrics parser changed: Maya 1.1.1 keeps the Strata `/metrics` contract
(`engine`/`live`/`requests`/`totals`/`hardware`), including the 0.1.38
fields. Maya-specific `engine_kind: glm-fast`, the `tiers` block and the
missing GGUF header (packed weights) render as documented unknowns.

## Maya panels

- **Throughput:** server-windowed `live.tok_s` while decoding (verified
  25.1 tok/s live) and mean prefill (37.8 tok/s on the verification request;
  122-358 tok/s across cached prompts). Peak decode 27.7 tok/s, tok/J from
  server hardware power.
- **Context:** 262,144-token capacity from `--max-context`; last-request
  prompt/reuse/generated counts.
- **Vision:** `vision CPU` from the folded strata-vision child (config
  `gpu: false`).
- **Layers:** GGUF header not readable for `--glm-pack` weights; per-layer
  placement stays not reported.
- **GPU placement:** GLM weights are attributed to the RTX 3090 through the
  existing direct evidence chain; the Radeon keeps the Qwen llama.cpp row.

## Verification

- 181 local tests passed (one new: the vision child folds without renaming the
  server); strict all-target clippy, `cargo fmt --check`, locked optimized
  build.
- Live on inference03 as ordinary `haja`, against Project Maya 1.1.1 serving
  GLM-5.3-Flash on the 3090 and Qwen3.8-27B on the Radeon:
  - Idle capture names the server `glm-5.3-flash · strata pid 1145011 ·
    vision CPU`, lists 2 models, and fills context, KV, expert cache, totals
    and newest-first request rows.
  - A 64-token verification request produced a live DECODE capture: model row
    `● decode 25.1 tok/s`, prefill 37.8 tok/s, peak 27.7, tok/J 0.09, server
    totals climbing (86 requests).
  - One 80-token request was sent to Maya as traffic; no service restart,
    configuration change or model reload was performed. The dashboard ran from
    a task-owned scratch binary with its own settings directory and `--log-db
    off`.

## Release/deployment status

- Release: https://github.com/hajekt2/llm-visuals/releases/tag/v0.10.0-hajek.4
- Tag: `v0.10.0-hajek.4`, prerelease on the fork only.
- Archive: `llm-visuals-0.10.0-hajek.4-linux-x86_64.tar.gz`, 2,437,961 bytes,
  sha256 `09c815197babbe2eb8eb70267b4fdaa6889d96381e1403f6da99f33ebd04f0d2`.
  Asset URL:
  https://github.com/hajekt2/llm-visuals/releases/download/v0.10.0-hajek.4/llm-visuals-0.10.0-hajek.4-linux-x86_64.tar.gz
- Installed version on inference03: `llm-visuals 0.10.0-hajek.4`; hash matches
  the release binary. The hajek.2 executable stays at
  `/usr/local/bin/llm-visuals.hajek2`.
- Pin: `ai_inference_llm_visuals_version: '0.10.0-hajek.4'` in
  `inventory/host_vars/inference03.yml`.
