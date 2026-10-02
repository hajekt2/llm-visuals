# Fork release publication receipt

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
