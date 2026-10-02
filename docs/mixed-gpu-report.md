# Mixed-vendor GPU fix: verification and release handoff

Verified on 2026-10-02. This report records the initial **local-only** handoff
on `fm/llm-visuals-mixed-gpu`; the unpublished state below refers to that
handoff. Subsequent firstmate steering authorized fork-only publication.
See [the verified publication receipt](mixed-gpu-release.md) for the live
release URL and deployable installer pin. No upstream issue or pull request
was made.

## Root cause and source base

AMD telemetry already exists in upstream v0.9.0. Upgrading to v0.10.0 alone
cannot fix this mixed-host bug.

Exact v0.10.0 locations:

- `src/gpu.rs:282-300`: `GpuBackend::detect` returns NVML immediately at line
  285; otherwise the exclusive `if / else if / else` at lines 288-298 selects
  NVIDIA, Intel, or AMD. AMD discovery at line 293 is unreachable when NVIDIA
  collection succeeds.
- `src/model_detect.rs:483-487`: AMD process collection only runs when the
  NVIDIA process list is empty. A running NVIDIA server therefore hides AMD
  process placement as well as AMD telemetry.
- The old weight/layer paths also treat vendor-local tensor-split ordinals as
  global dashboard indices, and unknown placement can claim every visible GPU.

Implementation started at upstream tag `v0.10.0`
(`c69106611ab8ad2af529f43450f1ea277c61bb84`). To satisfy the later required
fast-forward delivery contract, it was rebased onto local `main`
(`79f7a0bce789823c28e951fb57bf56399aac15d1`). **The final artifact includes
post-v0.10.0 upstream-main changes**, not just the tag: system-RAM graphs,
vision-projector detection/placement, and GPU-card spacing. Comparing the tag
with that main gives 11 changed files, 1,097 insertions and 73 deletions;
`src/gpu.rs` itself is unchanged between those upstream points. Both intents
were preserved during the rebase, including the vision imports and GPU
placement constraint.

## Changes

- Collect NVIDIA (NVML preferred, otherwise nvidia-smi), AMD sysfs/hwmon, and
  Intel xpu-smi independently. Use NVIDIA-first, AMD-card-number, Intel ordering.
- Retain vendor/backend, vendor-local ordinal, AMD PCI address, and unique
  global dashboard index. Mixed headers show `nvml+amd`, `smi+amd+xpu`, etc.
  Single-backend names and existing single-vendor ordinals remain unchanged.
- Cache mixed startup inventory. Failed polls retain unavailable device slots
  with zero current counters rather than stale values or renumbered neighbors.
  Unreadable AMD telemetry does not remove its card.
- Preserve the physical mixed-host flag even if one driver fails at startup,
  including headless NVIDIA PCI graphics functions without DRM cards, and
  after `--gpu` filtering. Unknown NVIDIA placement must not borrow AMD
  activity just because AMD is the only readable backend.
- Combine NVIDIA process records and AMD DRM records. Resolve open render
  nodes through PCI identity, not DRM numbering. Resolve container-renamed
  devices through `/proc/<pid>/root` and host device numbers; use host PIDs.
- Parse AMD fdinfo VRAM memory, including alternate `drm-total-vram` units.
  Deduplicate repeated descriptors of a client; keep placement with unknown
  memory when only the open render node is readable.
- Fold worker placement into its server. Live investigation also found Strata's
  Python HTTP adapter/native-engine split: recognize `--ple-gguf`, fold native
  NVIDIA VRAM onto the adapter, and show one Strata model. Strata is GPU/process
  telemetry only; no HTTP poller is started for it.
- Constrain weight/KV shares, layers, current GPU meters, PCIe counters, and
  bandwidth verdicts to serving devices. Clear stale counters after placement
  changes. Unknown mixed-host layer placement is shown as `?`, never invented
  as GPU 0 or driven by its utilization. Constrain the post-tag vision placement
  path too. Reject ordinary files masquerading as DRM render nodes.
- Add mixed-host fixtures, swapped DRM numbering, startup/poll failure,
  descriptor deduplication, container-root, worker-folding, affinity-mask,
  tensor-split, stale-counter, and rendering coverage. Resolve existing
  upstream stable-clippy lints so the strict requested check passes.

## Local verification

Final verified source commit: `521ce5d`.

Exact final checks:

```sh
cargo fmt
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
git diff --check
cargo build --release --locked
```

Results: **148 passed, 0 failed, 2 ignored**; strict clippy, formatting, diff
checks, and optimized release build passed. The two ignored tests need live
GPUs/servers and were also run successfully on inference03 below.

The initial strict-clippy run found 33 pre-existing upstream lints, consistent
with `.github/workflows/ci.yml` explicitly not having a clippy gate. Those were
resolved, not silently skipped. An initial Strata live check exposed duplicate
native/adapter detection and an unsupported full-topology assumption; those
were corrected as described above. The new unknown-layer rendering assertion
initially used a zero-layer fixture; initializing its layer count fixed that
test, and the final full suite passes. Native Linux checks were run; macOS,
Windows, and ARM cross-builds were not run.

`render::tests::mixed_gpu_panels_render_in_both_sizes_and_zoom_views` uses
Ratatui TestBackend at 150x46 and 100x30, all/perf/models/MoE views, checks both
cards and the combined backend label, checks an unavailable AMD card, and
compares the single-vendor GPU buffer before/after identity metadata for byte
identity. The swapped-DRM fixture produces identical global placement and
process memory with either Radeon card0/renderD128 or card1/renderD129.

## Real inference03 verification

The VM temporarily disappeared during another worker's approved CPU/RAM
and Radeon experiment. Firstmate authorized bounded two-minute retries; no
service was changed or restarted by this worker. The VM returned before the
extended deadline. A later cycle removed the scratch directory; only this
worker's scratch path was recreated, and host/OS/glibc were rechecked before
the exact final binary capture.

Builder `ai` and target `inference03` both run Ubuntu 24.04.5 LTS, x86_64,
glibc `2.39-0ubuntu8.9`. The exact final binary was copied only to
`/tmp/llm-visuals-mixed-gpu-1790940835/` and run there with a timeout and logging
off, in an isolated tmux socket. No system-wide installation occurred.

DRM numbering **actually swapped during the experiment**:

| Boot | Radeon | RTX 3090 |
| --- | --- | --- |
| Before second reboot | card0, PCI `0000:01:00.0`, vendor `0x1002` | card1, PCI `0000:02:00.0`, vendor `0x10de` |
| Final verification | card1, PCI `0000:01:00.0`, vendor `0x1002` | card0, PCI `0000:02:00.0`, vendor `0x10de` |

The dashboard remained NVIDIA **G0**, AMD **G1**. AMD busy/memory sysfs and
hwmon temperature/power were readable. Early idle readings included 7 W,
29 C and a 294 W existing cap; final GPU collection showed approximately
22,228 MiB used on the Radeon. No cap, clock, GPU lock, benchmark, or service
configuration was touched.

The llama.cpp router/worker are **root-owned**. Ordinary `haja` can read GPU
sysfs but cannot read their `/proc/<pid>/fd` or fdinfo: the non-elevated smoke
check correctly gave unknown placement, not NVIDIA ownership. Read-only
`sudo -n` access was available and used for placement/TUI verification.
Production attribution requires the same process UID or equivalent read
permission; run the dashboard with appropriate access when observing these
root-owned containers. No permissions were weakened. The exact final binary
was also run as ordinary `haja`: it displayed `GPU placement unknown` and `?`
on all 65 Qwen layers, not NVIDIA G0.

Exact live-test commands (on the target, using the copied native test binary):

```sh
sudo -n timeout 60 /tmp/llm-visuals-mixed-gpu-1790940835/live-tests \
  --ignored --exact model_detect::tests::detects_live_servers --nocapture
sudo -n timeout 15 /tmp/llm-visuals-mixed-gpu-1790940835/live-tests \
  --ignored --exact gpu::live_tests::collects_live_gpu_stats --nocapture
```

Both passed. Safe placement output:

```text
detected: Qwen3.8-Flash-Next-GSQ-RCO-IQ3_S-00002-of-00002 (PID 1977 · strata · GPU 0 · 23412 MB) port=Some(8098) layers=0 heads=0 experts=0/0
detected: Qwen3.8-27B-UD-Q4_K_M (PID 1705 · llama.cpp · GPU 1 · 22201 MB) port=Some(47703) layers=65 heads=24 experts=0/0
```

Port 47703 is the router's host-visible llama.cpp child; the authorized request
was sent to the router at **172.16.0.73:8080**, using `Qwen3.8-27B-Fast`.
Exactly **one** short inference request was made: temperature 0, max_tokens 48,
asking for numbers 1 through 12, thinking disabled. It returned HTTP 200 in
1.030 seconds, with 30 prompt and 27 completion tokens. No request of any kind
was sent to Strata on :8098; a unit test also verifies Strata gets no pollers.

The 100 ms read-only sysfs sampler observed:

- Radeon utilization: **0% -> 100% -> 0%**.
- Radeon VRAM: **22,228.039 -> 22,231.137 MiB** (+3.098 MiB). The large
  weight/KV allocations were already resident, so this small delta is expected.
- During-request screen: Qwen selected, Radeon G1 at 74% after UI smoothing,
  RTX G0 at 0%, model tenants G0 `<1>` / G1 `<2>`, all 65 Qwen layer tiles on G1.
- Radeon legend: approximately 15.3 GiB estimated weights and 6.4 GiB KV/other
  residual, with 21.7 GiB total used. NVIDIA activity was not used for Qwen's
  layers, power, PCIe, or bandwidth verdict.

Full, unedited text captures and samples are committed under
[`mixed-gpu-verification/`](mixed-gpu-verification/):

- [150x46 before](mixed-gpu-verification/live-before-150x46.txt)
- [150x46 during the request](mixed-gpu-verification/live-during-150x46.txt)
- [150x46 after](mixed-gpu-verification/live-after-150x46.txt)
- [100x30 dashboard](mixed-gpu-verification/live-100x30.txt)
- [100x30 perf](mixed-gpu-verification/live-perf-100x30.txt)
- [100x30 MoE](mixed-gpu-verification/live-moe-100x30.txt)
- [100x30 model comparison: Strata G0, Qwen G1](mixed-gpu-verification/live-models-100x30.txt)
- [Exact final binary checksum and live 150x46 capture](mixed-gpu-verification/live-final-binary-150x46.txt)
- [Final non-root capture: unknown placement, not G0](mixed-gpu-verification/live-final-unprivileged-150x46.txt)
- [Request result and sysfs samples](mixed-gpu-verification/live-request-samples.txt)
- [Live placement tests](mixed-gpu-verification/live-placement-final.txt)

The request-time capture precedes the final startup-failure, render-node
validation, and unknown-layer-label guards; the exact final packaged binary
was then copied, checksum-compared, and smoke-captured in both permission
contexts without another inference request. Healthy-host behavior and attribution
were identical.

Limitations: the existing metadata reader did not resolve full Strata topology
from the supplied PLE source, and no Strata counter adapter is implemented.
Its zero/default live-rate fields are not measured
inference counters. Intel mixed-host visibility masks alone are deliberately
not treated as usage evidence. In the compact 100x30 all-layout, upstream-main
layout pressure hides throughput numeric rows and card VRAM detail; the
verified perf/model-comparison views expose them. The short MTP request also
showed 13 generated tokens in the UI versus 27 in the API response; throughput
accounting was not changed or claimed verified here. Inference success and GPU
activity were checked independently through the API and sysfs. No unrelated
layout/counter overhaul was attempted.

## Prepared release assets and installer values

The public upstream asset list was checked via:

```sh
gh-axi api /repos/DingoOz/llm-visuals/releases/tags/v0.10.0 \
  --jq '{url: .html_url, assets: [.assets[] | {name: .name, url: .browser_download_url}]}'
gh-axi release download v0.10.0 --repo DingoOz/llm-visuals \
  --pattern llm-visuals-0.10.0-linux-x86_64.tar.gz --dir target/verification/upstream
```

Both upstream and fork tar listings contain exactly `llm-visuals` at the
archive root. This matches `.github/workflows/release.yml` and
`/home/haja/work/homelab-iac/roles/ai_inference/tasks/main.yml`, which extracts
into a versioned directory and copies its root binary. The first asset lookup
used unsupported gh-axi `--json/--jq` release-view flags; after consulting help,
the supported API command above succeeded.

Packaging/check commands:

```sh
tar -C target/release -czf dist/llm-visuals-0.10.0-hajek.1-linux-x86_64.tar.gz llm-visuals
(cd dist && sha256sum llm-visuals-0.10.0-hajek.1-linux-x86_64.tar.gz \
  > llm-visuals-0.10.0-hajek.1-linux-x86_64.tar.gz.sha256)
(cd dist && sha256sum -c llm-visuals-0.10.0-hajek.1-linux-x86_64.tar.gz.sha256)
```

Checksum verification: **OK**.

Local prepared assets, in this worktree:

- `dist/llm-visuals-0.10.0-hajek.1-linux-x86_64.tar.gz`
- `dist/llm-visuals-0.10.0-hajek.1-linux-x86_64.tar.gz.sha256`

Tarball SHA-256:
`818c8ef95ee44e608330fc72fc92d8b4d874ab563c9bf844871847d3af5649fd`

Binary SHA-256, matching the target scratch copy:
`6bf70ae51b824e731b23620342e1fdc49110636e0dd151655f6108129c49f051`

**Not published.** Intended release/tag URL after firstmate publishes:
`https://github.com/hajekt2/llm-visuals/releases/tag/v0.10.0-hajek.1`

Intended asset URL:
`https://github.com/hajekt2/llm-visuals/releases/download/v0.10.0-hajek.1/llm-visuals-0.10.0-hajek.1-linux-x86_64.tar.gz`

Exact homelab values **after publication**:

```yaml
ai_inference_llm_visuals_version: '0.10.0-hajek.1'
ai_inference_llm_visuals_sha256: 818c8ef95ee44e608330fc72fc92d8b4d874ab563c9bf844871847d3af5649fd
ai_inference_llm_visuals_release_base_url: https://github.com/hajekt2/llm-visuals/releases/download
```

The tarball/sidecar are ignored build artifacts, not Git blobs. Firstmate should
retain/copy `dist/` before recycling this worktree, review the source-base choice,
publish these exact assets to the owner's fork, and only then change the pin.
No homelab files were edited here.

## Upstream-ready description (not posted)

On hosts with more than one GPU vendor, selecting one telemetry backend hides
otherwise supported devices and can attribute one vendor's activity to a
server using another. Collect each vendor independently, preserve backend-local
identity behind stable global indices, and retain unavailable cards when one
collector fails. Combine NVIDIA process records with PCI-mapped AMD DRM
render-node/fdinfo records, including container-root device resolution and
client deduplication, then constrain model memory, layers and counters to
observed serving GPUs. Mixed-host fixtures cover independent backend failures,
swapped DRM numbering, filtered views, and single-vendor output preservation.
The core fix is ready to offer upstream later with owner approval; Strata
detection and unrelated existing-lint cleanup should be reviewed separately
from the mixed-telemetry/attribution patch.
