# Strata JSON metrics negotiation: v0.10.0-hajek.5

## Failure and cause

On inference03, llm-visuals detected Strata 0.1.44.1 and the CPU vision
encoder, but showed “Metrics unavailable.” The Strata `/metrics` endpoint was
reachable and returned valid JSON to `curl`.

The dashboard sent `Accept: application/json, text/plain, */*`. Strata's
content negotiation selected Prometheus text (`# HELP vllm:...`) for that
header, so the JSON parser rejected the response. Asking for
`Accept: application/json` returns Strata's JSON metrics.

There was also a reachability gap: the container publishes port 8098 on the
LAN address only, while its wildcard config makes llm-visuals poll loopback.
The homelab Compose config now publishes the same port on loopback as well. The
LAN binding remains unchanged.

## Fix

- Add `http_get_with_accept`, while preserving the existing default behavior
  for llama.cpp and other callers.
- Have Strata's metrics poller request only `application/json`.
- Add an HTTP-level regression test that verifies the request header and parses
  a captured Strata JSON fixture.

## Verification

- Reproduced content negotiation on inference03: the broad Accept header
  returned Prometheus text; JSON-only returned the expected Strata object.
- Built the fixed binary and ran it in a private tmux session against
  inference03. The dashboard populated Strata throughput, GPU, context, cache,
  server totals and request history. No inference request was sent.
- `cargo fmt --check`, `cargo test --locked`, `cargo clippy --all-targets
  --locked -- -D warnings` and `cargo build --release --locked` passed.
- The public prerelease `v0.10.0-hajek.5` is published and installed on
  inference03. A private tmux dashboard session showed the Strata model and
  both GPUs; the session was closed without sending an inference request.
