# Recorded metrics fixtures

`strata-idle.json`, `strata-prefill.json` and `strata-busy.json` are unmodified
read-only `/metrics` captures from a Strata 0.1.31 service. They contain counters
and hardware telemetry only, not prompts, generated text or credentials. Busy
traffic includes an existing client request; the prefill capture was collected
while a short validation chat was running. No service settings were changed.

Contract cross-checked against upstream Strata commit
`aeb35bed019bc2031d25007037b1249b5249b9ca`:

- [Monitor documentation](https://github.com/Niko1221/Strata/blob/aeb35bed019bc2031d25007037b1249b5249b9ca/docs/DETAILS.md)
- [Service.metrics and request summaries](https://github.com/Niko1221/Strata/blob/aeb35bed019bc2031d25007037b1249b5249b9ca/serve/server.py)
- [Hardware units and aggregation](https://github.com/Niko1221/Strata/blob/aeb35bed019bc2031d25007037b1249b5249b9ca/serve/telemetry.py)
- [Monitor rendering and context arithmetic](https://github.com/Niko1221/Strata/blob/aeb35bed019bc2031d25007037b1249b5249b9ca/serve/web/app.js)

`strata-error.json` is a **synthetic** error response used by the mock HTTP 503
and parser tests. No errors or service restarts were induced on the live server.
Unreachable tests use a closed ephemeral loopback port. Unit tests also mutate
small JSON samples to verify missing, renamed, negative and wrong-type fields.

`strata-metrics-0.1.38.json` is an unmodified read-only capture from
`http://172.16.0.73:8098/metrics` on 2026-10-03 at Unix time
`1791024596.9947155`. It contains only metrics, not prompts or responses.
It records an idle engine, 12 recent request summaries, 61 completed requests,
nullable live fields, draft counters and server-side hardware histories.
Tests explicitly mark synthetic reading/generating transitions derived from
this schema; no traffic was sent to obtain this fixture.
`strata-metrics-0.1.38-unloaded.json` is a later read-only capture of the same
service after another client's failed request. It verifies nullable request
fields and last-request context while the engine is unloaded. The dashboard
worker did not unload, restart or generate traffic against the service.

`strata-metrics-0.1.35.json` and `strata-config.json` are synthetic contract
fixtures from the reviewed reconciliation work. `strata-metrics-upstream.json`
is its older-shape compatibility example. They are not evidence of live
0.1.38 traffic. Slow-handler and failure/recovery tests run on loopback only.

The existing SGLang/vLLM/XPU fixtures retain their original provenance.
