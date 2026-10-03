# Strata Flash-Next integration: v0.10.0-hajek.3

## Problem and fix

Inference03's installed `0.10.0-hajek.2` recognized both models and GPUs, but
had no Strata counter adapter. Its poller deliberately excluded Strata. This
branch starts at tag `v0.10.0-hajek.2` and ports `a1739a3`'s adapter, with
reviewed portions of reconciliation commit `b1ef450`, preserving mixed-GPU
placement and the llama.cpp path.

Additional integration fixes:

- Start a JSON `/metrics` poller for process-detected Strata launchers.
- Preserve the published bind IP as well as the container port. On inference03,
  Strata's Docker proxy listens on `172.16.0.73:8098`, not loopback.
- Read the native GGUF shard from `--config` through the launcher's mount
  namespace. Do not use the embedding-only `--ple-gguf` shard as full topology.
- Keep the last successful snapshot on failed or hung scrapes, with an amber
  `STALE` badge and age. Connection/read waits are bounded at 500/1500 ms.
  Stale samples do not extend throughput or acceptance histories.
- Use optional completed-request draft counts for acceptance. Missing/null
  counts, zero denominators, TTFT, verification steps and parked occupancy
  remain unknown. No per-layer device assignments or routing are simulated.
- Show last-request context when idle or unloaded, explicitly distinguished
  from resident KV occupancy. Keep the model comparison and zoom keys working.

## Flash-Next panels

- **Throughput:** server-windowed `live.tok_s` and
  `live.prefill_tok_s_mean`; zero while idle/unloaded. Recent rows preserve
  historical decode and computed prefill rates. Server totals show request
  and generated-token counts.
- **Context:** prompt plus generated tokens during a live request; last request
  prompt plus output otherwise. Capacity is 262,144 tokens on this service.
- **Layers/experts:** native GGUF reports 48 layers, 24 heads, 2 KV heads and
  10 of 512 experts per token. The layer zoom labels these as metadata only.
  Per-layer placement and routing are not exported by Strata.
- **GPU placement:** retain both local driver cards and existing placement
  provenance. Weight/KV memory split stays unknown for Strata. A remote
  endpoint uses server hardware instead of the monitoring host's GPUs.
- **Requests/speculation:** newest-first server request summaries, including
  finish/error reason, prompt/reuse/output counts, prefill/decode rates,
  prompt time, draft acceptance and wider-screen expert-cache/I/O details.

## Verification before publication

- 180 local tests passed; two hardware-dependent tests passed on inference03
  as ordinary `haja`.
- `cargo fmt --check`, strict all-target clippy, locked optimized build and
  diff checks passed. Cross-OS/ARM compilation is not locally verified.
- Saved unmodified 0.1.38 idle and unloaded fixtures; synthetic busy/prefill
  transitions are explicitly marked in tests. Loopback tests cover HTTP 503,
  hung handlers, retained stale values and automatic recovery.
- [Scratch binary capture](strata-flash-verification/scratch-user.txt) covers
  both models, both GPU cards, comparison, Flash layer/perf zoom and 100x30.
- [Live test output](strata-flash-verification/live-tests.txt).
- [Reproduction script](strata-flash-verification/capture.sh) sends no inference
  requests and uses its own terminal socket and settings directory.

At capture time the Strata service reported **unloaded**, following another
client's error request at Unix time `1791024667.742399`. Its GPU allocation was
gone. The dashboard correctly showed zero current rates, 135,753 / 262,144
last-request context, 62 server requests, 48,979 generated tokens and earlier
successful request rates. Live busy rates were not revalidated in this service
state; parser/performance tests cover them. No service restart, model reload,
configuration change or inference traffic was performed by this worker.

## Release/deployment status

Prepared release name: `v0.10.0-hajek.3`. Publication, independent asset download
verification and installation are pending. The intended destination is the
`hajekt2/llm-visuals` fork only, as a prerelease, not latest stable. Installation
will preserve `/usr/local/bin/llm-visuals.hajek2` before replacing the executable.
