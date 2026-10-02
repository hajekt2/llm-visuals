# Fork release receipts

## v0.10.0-hajek.2: current release

Prepared on 2026-10-02; publication and independent download verification are
pending. Firstmate steering `001.msg` authorized publishing this follow-up to
the owner's fork after ordinary-user live validation. No upstream contact or
publication, homelab deployment, service restart or configuration change is
part of this worker's release.

- Release URL: https://github.com/hajekt2/llm-visuals/releases/tag/v0.10.0-hajek.2
- Version: `0.10.0-hajek.2`
- Tag: `v0.10.0-hajek.2`
- Branch: `fm/llm-visuals-mixed-gpu-v2`
- Source change: `c202e230cc906803ef9a24ba5d34b41eba790b0f`
- Intended state: published prerelease, not the fork's latest stable release.

Asset URL:
https://github.com/hajekt2/llm-visuals/releases/download/v0.10.0-hajek.2/llm-visuals-0.10.0-hajek.2-linux-x86_64.tar.gz

Sidecar: same URL plus `.sha256`.

Archive SHA-256:
`3ac0884102be2bfd7dbe3887c18612cf88fc90c409968639f17b2cc3eaafea38`

Contained executable SHA-256, matching the live-verified scratch binary:
`1c8cc7ac662b41d622563d21c7bacd40b2f6e00032d22505ead7a7660f32a977`

The 2,383,764-byte archive contains exactly `llm-visuals` at its root. Build host
and target are Ubuntu 24.04/glibc 2.39, Linux x86_64. Older glibc is unverified.
Tar ownership is fixed to root and its mtime is source-change commit epoch
`1790948177`; gzip stores no source filename, so identical inputs reproduce
this archive. The `.sha256` names the archive, not the executable.

### Change and operational behavior

Running as ordinary `haja`, root-owned llama.cpp descriptors are inaccessible.
The new version keeps driver/descendant placement direct when readable; when
only one unclaimed GPU with VRAM and one denied server remain, it shows Radeon
placement **inferred** with the reason. Ambiguity stays unknown with the denied
procfs path. Overrides are **configured**, never disguised as measured evidence:

```sh
llm-visuals --server-gpu 8080=pci:0000:01:00.0 --log-db off
LLM_VISUALS_SERVER_GPU='8080=amd,8098=nvidia' llm-visuals --log-db off
```

CLI wins over environment per port; router mappings reach loaded children.
Inference does not prove ownership, and per-process memory without counters is
`not measured`. The GPU panel's VRAM remains measured; its weight/KV split is
estimated. No elevated helper or procfs permission change is required or added.
`--version` and `-V` now return `llm-visuals 0.10.0-hajek.2`.

Verified locally: 160 tests passed, 2 live-only tests ignored; strict clippy,
formatting, diff checks and optimized build passed. Both ignored tests then
passed on inference03. Root direct, ordinary automatic inferred, explicit
PID/endpoint inferred and public-router configured paths were captured idle
without any inference request. Full evidence and limitations are in
[the report](mixed-gpu-report.md#v0100-hajek2-follow-up-ordinary-user-deployment-2026-10-02).

### Exact new homelab pin after publication

```yaml
ai_inference_llm_visuals_version: '0.10.0-hajek.2'
ai_inference_llm_visuals_sha256: 3ac0884102be2bfd7dbe3887c18612cf88fc90c409968639f17b2cc3eaafea38
ai_inference_llm_visuals_release_base_url: https://github.com/hajekt2/llm-visuals/releases/download
```

The existing role's URL scheme and root executable layout are unchanged.
No role-task change is needed; no homelab file was edited or deployed here.

## v0.10.0-hajek.1: historical publication receipt

Published and download-verified on 2026-10-02, following firstmate steering
message `002.msg`, which superseded the earlier local-only handoff restriction.
Only the public fork `hajekt2/llm-visuals` was modified. No upstream issue,
pull request, push, or publication was made; no inference service was changed.

## Release

- URL: https://github.com/hajekt2/llm-visuals/releases/tag/v0.10.0-hajek.1
- Version: `0.10.0-hajek.1`
- Tag: `v0.10.0-hajek.1`
- Tag target: `95b5c89eeac8f564c38a051b0e9a151a392d2d8b`
- State: published, prerelease, not the fork's latest stable release.
- Branch: `fm/llm-visuals-mixed-gpu`, pushed to remote `fork` without force.

Asset URL:
https://github.com/hajekt2/llm-visuals/releases/download/v0.10.0-hajek.1/llm-visuals-0.10.0-hajek.1-linux-x86_64.tar.gz

The companion asset has the same URL plus `.sha256`.

Tarball SHA-256:
`818c8ef95ee44e608330fc72fc92d8b4d874ab563c9bf844871847d3af5649fd`

Contained binary SHA-256:
`6bf70ae51b824e731b23620342e1fdc49110636e0dd151655f6108129c49f051`

The tarball is 2,366,366 bytes and contains exactly one executable:

```text
llm-visuals
```

There is no enclosing directory. This is the exact binary previously exercised
on inference03, built on Ubuntu 24.04 / glibc 2.39; older glibc versions are not
verified.

## Published-asset verification

At publication, the branch and tag were confirmed with `git ls-remote fork`
and both resolved to the release's stated source commit. This documentation
receipt subsequently advances the branch without moving the release tag.
Release metadata confirms `draft: false`,
the two expected asset names, and GitHub's archive digest matches the pin.

```sh
gh-axi release download v0.10.0-hajek.1 --repo hajekt2/llm-visuals \
  --pattern 'llm-visuals-0.10.0-hajek.1-linux-x86_64.tar.gz*' \
  --dir target/verification/published
(cd target/verification/published && \
  sha256sum -c llm-visuals-0.10.0-hajek.1-linux-x86_64.tar.gz.sha256)
```

Result: **OK**. Independent hashing of the downloaded archive confirms the
exact pin above. Archive inspection confirms the root-level executable and
its hash matches the live-verified binary. The verification did not merely
check local `dist/` artifacts.

## Exact homelab pin

Put these values in the appropriate inventory scope:

```yaml
ai_inference_llm_visuals_version: '0.10.0-hajek.1'
ai_inference_llm_visuals_sha256: 818c8ef95ee44e608330fc72fc92d8b4d874ab563c9bf844871847d3af5649fd
ai_inference_llm_visuals_release_base_url: https://github.com/hajekt2/llm-visuals/releases/download
```

**No role-task change is needed.** The existing
`roles/ai_inference/tasks/main.yml` already interpolates the configurable release
base, `/v<version>/`, and the Linux x86_64 archive name. Its checksum-pinned
`get_url` downloads the archive; `unarchive` extracts the root binary into
`/usr/local/src/llm-visuals-0.10.0-hajek.1/`; the copy task installs that binary
at `/usr/local/bin/llm-visuals` with root ownership and mode 0755. The default
base points upstream, so the inventory override above is essential.

No homelab files were edited or deployment performed by this worker. Process
attribution still needs appropriate read access to root-owned llama.cpp
containers. See [the original verification report](mixed-gpu-report.md) for
tests, screenshots, source-base differences, and known limitations.
