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

The existing SGLang/vLLM/XPU fixtures retain their original provenance.
