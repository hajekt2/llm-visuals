# Strata backend reconciliation

## Sources and choice of baseline

Compared on 2026-10-02:

- Upstream main, v0.10.0: `79f7a0bce789823c28e951fb57bf56399aac15d1`, confirmed with `gh-axi api repos/DingoOz/llm-visuals/commits/main --jq .sha`.
- Upstream [feature/strata](https://github.com/DingoOz/llm-visuals/tree/feature/strata): `8a1492d080cf168928339a9828734d3a824e6747`. Read its adapter, detection, poll loop, renderer changes and metrics fixture via `git show`/`git diff`.
- Fork `fm/llm-visuals-strata`: `9ba5660d84faa5d94b3af19ee5d6b409e68d89f8`.
- Acceptance contract: https://github.com/DingoOz/llm-visuals/issues/32, read with `gh-axi api repos/DingoOz/llm-visuals/issues/32 --jq .body`.
- Reference video: https://x.com/needmorevram/status/2105826361378259041, inspected the supplied `f03.png` frame. It shows two RTX 5060 Ti cards, two model rows and the performance layout.

The new work starts on upstream main, then reapplies the fork adapter and incorporates the upstream detection/config/worker ideas. This preserves system RAM graphs, vision placement support for other engines, context-speed view and GPU card spacing. There is one Strata metrics adapter, not parallel implementations. No upstream contact or pull request was made.

## Feature comparison

| Feature | Upstream main | Upstream feature/strata | Fork 9ba5660 | Reconciled backend |
|---|---|---|---|---|
| Detection | No Strata | Python HTTP launcher; JSON probe; port 8095 | Launcher/module and JSON probe; incorrectly defaulted to 8080 | Launcher/module; reject Docker/shell wrapper false positives; correct 8095 default; backend filter |
| Explicit remote/container endpoint | Generic engine probes | Generic JSON probe | Dedicated `--backend strata --endpoint`, metrics only | Retained; only `/metrics` is needed to attach explicitly |
| Config/model facts | No Strata | `model_name`, first `--native` GGUF, `--max-context` from JSON | Endpoint model/window only | Upstream config facts plus server-namespace path anchoring and existing GGUF reader |
| GPU attribution | No native Strata folding | Fold direct native child memory to Python PID | No native child folding | Same shared worker accumulator as SGLang; sum allocations across cards; retain PID in header/status |
| Live stats | No Strata | Slot-like counters; prefill progress includes reused prefix; rates reconstructed from polls | Actual server-windowed decode and mean prefill; unknown fields preserved | Retain fork mapping; 400 ms minimum HTTP cadence; backoff and immediate stale-snapshot clearing |
| Request log | No Strata | Generic completion reconstructed from polling edges | All recent server summaries, newest first; prompt time, not fabricated TTFT | Retained; optional per-request draft acceptance; shorter core columns fit 100-wide terminals |
| Speculative panel | No Strata | Explicit unavailable acceptance fallback | Settings text only, acceptance unavailable | Shared SPECULATIVE gauge/history from 1.5-second deltas; server-lifetime sums; fallback for older/null fields |
| Multi-GPU/models | Mature shared GPU/model panels | Shared panels after child folding | Remote hardware aggregate; Strata bypasses per-card panels | Per-card driver graphs locally, aggregate remote telemetry remotely; shared model strip/comparison |
| Layer/expert views | GGUF facts; labelled visualization/estimates | Shared generic layer/expert rendering | Views all redirected to Strata summary | `h`/`m`: GGUF architecture and expert-cache facts, no invented routing/placement |
| Vision | Encoder placement for other engines | Branch predates current placement work | No Strata placement source | Latest main preserved; Strata image support marked with unknown placement, false clears stale vision |
| Docker paths/ports | Generic model path/port re-anchoring | Config read directly from resolved cwd; no explicit config namespace re-anchor or Strata port mapping | Explicit published endpoint works; no config facts | Config/GGUF via `/proc/<pid>/root` or `cwd`; published-port resolution reused; explicit URL remains permission/NAT fallback |

### Why keep the fork's live mapping?

`live.prompt_read` includes a reused prefix. Treating its delta as freshly computed prefill produces spurious rates. Strata already reports `live.tok_s` and `prefill_tok_s_mean`; these are the numbers used by its Monitor. Finished `prompt_ms` is not measured TTFT. The fork's request summaries preserve server measurements even when requests start and end between polls. Nullable/absent metrics remain unknown rather than pretending to be real zeros.

### Why not copy the video's weights/KV split or tokens/step?

The shared generic memory estimator uses file size, tensor split and remaining GPU use. Strata's GGUF includes experts that can reside off-card, while its VRAM also contains expert caches and allocator arenas. Driver-reported total memory cannot separate these categories. The video labels roughly all VRAM as weights and zero as KV; that is not evidence of a measured Strata allocation breakdown. This backend shows real total VRAM and explicitly marks weights/KV unknown.

Similarly, 0.1.35's two draft-token counters do not report verification steps. Neither depth (`mtp_max`) nor generated output recovers actual tokens/step. The acceptance percentage and accepted/offered totals are supported; the video's 2.78 tokens/step and steps/s are deliberately omitted rather than estimated.

## Rendering versus the video

| Video feature | Result / remaining limit |
|---|---|
| Engine and PID | Shown for local process discovery. URL-only remote attachment shows `strata endpoint`, not an invented remote PID. |
| 48 layers, 24 heads, 2 KV; MoE 10/512; engram 3-gram | Existing GGUF parser/header rendering now reachable through Strata config discovery. Exact values require that model's readable GGUF; 0.1.31 `/metrics` does not publish them. No remote filesystem/model download is attempted. |
| Vision placement | Strata reports image support, not encoder device placement. No `vision G3 (CUDA0)` claim is possible from the observed metrics. Other backends retain upstream placement support. |
| Two per-GPU panels and system RAM graph | Local detected processes use the latest shared per-card graph/spacing and host RAM graph. Remote metrics supply aggregate GPU/RAM histories, not independent per-card histories. The live server reports one GPU, not the video's two-card hardware. |
| CONTEXT bar | Now shared, with live or last-completed request context. Live prefix reuse remains unknown until completion. It is not total parked KV occupancy. |
| REQUESTS table | Server summaries, measured prompt/decode rates, prompt duration, finish reason, optional acceptance. Prompt duration is labelled prompt time, not TTFT. |
| SPECULATIVE 88%, 729/828 | Supported by synthetic 0.1.35 fixture and future server reports. Counts land at completion only; no recent completions give an undefined window percentage. Current 0.1.31 retains the exact unavailable fallback. |
| Layers/expert heatmaps | Architecture and expert-cache size/slots/hit facts are available. Actual layer placement, per-layer utilization, attention and expert identities/routing are not reported. No synthetic routing is presented as real. |
| Second model row | Existing latest-main model strip retained and tested with two models. `--backend auto` lists other engine types too; `--backend strata` intentionally filters to Strata. An explicit remote URL does not discover other remote processes. |

`p` most closely matches the video's graph-focused layout. `a` adds detailed Strata cache/engine facts on taller terminals. The earlier `docs/strata-idle.txt` and `docs/strata-busy.txt` are historical captures of 9ba5660, not current layout snapshots. Renderer tests can emit current synthetic text layouts with `STRATA_CAPTURE=1` into ignored `target/` files.

## Read-only verification and handoff

Execution host: `ai`, user `haja`, Linux 6.8.0-142-generic, SSH session, not WSL or a container. Route check:

```sh
ip route get 172.16.0.73
# 172.16.0.73 dev ens160 src 172.16.0.22
```

No local Tailscale executable was available; the LAN route and bounded successful HTTP request established access directly.

```sh
curl --max-time 5 -fsS http://172.16.0.73:8098/metrics
ssh -o BatchMode=yes -o ConnectTimeout=5 inference03 \
  'hostname; whoami; uname -sr; nvidia-smi --query-gpu=index,name,memory.used,memory.total,utilization.gpu --format=csv,noheader; command -v llm-visuals'
```

The metrics request succeeded and reported Strata 0.1.31, one RTX 3090 and no draft-total fields. SSH succeeded, confirmed the execution host/user/OS and returned RTX 3090 telemetry (23552 / 24576 MiB used, 99% utilization at that instant). An existing `/usr/local/bin/llm-visuals` is present there. The new binary was **not** copied or installed there: this worker's delivery contract prohibits modifications outside its worktree. The new local binary was exercised against the real endpoint in a 150x46 PTY for four seconds, quit with `q`, exit 0. Its captured screen showed the remote GPU/RAM histories, CONTEXT, request table and the exact old-server acceptance fallback. No inference, benchmark, restart, reconfiguration or server write was performed.

Run the built version from this worktree:

```sh
./target/debug/llm-visuals --backend strata --endpoint http://172.16.0.73:8098 --log-db off
# Press p for the video-like graph layout.
```

After an authorized installation on the monitoring host:

```sh
llm-visuals --backend strata --endpoint http://172.16.0.73:8098 --log-db off
```

After an authorized installation of this build on inference03, run in an interactive SSH terminal:

```sh
ssh -t inference03 'llm-visuals --backend strata --endpoint http://172.16.0.73:8098 --log-db off'
# Or, for local process/config detection and all engines:
ssh -t inference03 'llm-visuals --log-db off'
```

Those remote commands are handoff instructions, not claims that the existing remote binary has been upgraded. Reading container config/GGUF requires process-namespace permissions; without them the explicit URL still works, but architecture facts stay unavailable.

### Local checks

| Exact command | Result |
|---|---|
| `STRATA_CAPTURE=1 cargo test --locked` | Exit 0: 148 passed, 0 failed, 2 ignored; 5.34 seconds test runtime |
| `cargo fmt --check` | Exit 0 |
| `cargo clippy --all-targets --locked` | Exit 0; 34 existing style warnings in test target (31 shared with binary); no new Strata lint warnings |
| `cargo build --locked --all-targets` | Exit 0, no compiler warnings |
| `git diff --check` | Exit 0 |

The two existing ignored tests require real local GPUs/servers. They were not
forced on this worker host. macOS/Windows/ARM cross-builds and CI were not run;
new Linux namespace behavior is cfg-gated with non-Linux fallbacks. Real Docker
auto-discovery was not exercised against the protected remote host. Unit tests
cover launcher recognition, native memory folding across two cards, process-root/
cwd path anchoring, explicit/auto JSON discovery, HTTP failure/recovery, both old
and modern metric shapes, nullable/zero draft counts, reset and simulated restart.
Layout tests cover 150x46, 100x30 and 40x12 terminals, performance/layer/expert/
bandwidth views, two-model listing and local versus remote GPU rendering.

The initial test run found one obsolete renderer assertion that expected every
zoom key to show the same hardware panel. It was replaced with per-view assertions;
all tests pass in the final run. Existing Clippy warnings were inspected; unrelated
style cleanup was deliberately excluded from this adapter reconciliation.

This worker ships only the local `fm/llm-visuals-strata-reconcile` branch. It did not push, open a pull request, merge, run a pipeline or replace `~/.cargo/bin/llm-visuals`. Firstmate owns installation (including the requested `.prev` backup) and any later public-fork push to `fm/llm-visuals-strata-v2`.
