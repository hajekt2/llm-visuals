//! Strata's JSON `/metrics` adapter. Unlike llama.cpp, its rate is already
//! windowed by the server. Preserve unknown fields as unknown, not real zeroes.
use crate::observe::{http_get, HttpAuth, LiveStats};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Strata HTTP default, not llama.cpp's 8080.
pub const DEFAULT_PORT: u16 = 8095;

#[derive(Debug, Clone, Default)]
pub struct StrataMetrics {
    pub engine: Value,
    pub live: Value,
    pub totals: Value,
    pub hardware: Value,
    pub hardware_static: Value,
    pub requests: Vec<Value>,
    pub history: Value,
}

/// Numeric fields can disappear or become null between engine versions.
/// Reject negative/non-finite values and strings rather than inventing data.
pub fn number(v: &Value, key: &str) -> Option<f64> {
    v.get(key)?.as_f64().filter(|n| n.is_finite() && *n >= 0.0)
}

/// 0.1.35 counts are optional (including explicit null), never estimated.
/// A zero denominator is reported but has no defined acceptance percentage.
pub fn draft_counts(v: &Value) -> Option<(u64, u64)> {
    let offered = v.get("drafts_offered")?.as_u64()?;
    let accepted = v.get("drafts_accepted")?.as_u64()?;
    (accepted <= offered).then_some((offered, accepted))
}

pub fn draft_acceptance(v: &Value) -> Option<f32> {
    let (offered, accepted) = draft_counts(v)?;
    (offered > 0).then(|| accepted as f32 / offered as f32)
}

pub fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str()
}

pub fn parse_metrics(body: &str) -> Option<StrataMetrics> {
    let v: Value = serde_json::from_str(body).ok()?;
    if v.get("error").is_some() {
        return None;
    }
    let engine = v.get("engine")?.as_object()?;
    let live = v.get("live")?.as_object()?;
    // An OpenAI /models reply or another engine's JSON isn't Strata metrics.
    if !engine.contains_key("max_context") && !engine.contains_key("kv") {
        return None;
    }
    Some(StrataMetrics {
        engine: Value::Object(engine.clone()),
        live: Value::Object(live.clone()),
        totals: v.get("totals").cloned().unwrap_or_default(),
        hardware: v.get("hardware").cloned().unwrap_or_default(),
        hardware_static: v.get("hardware_static").cloned().unwrap_or_default(),
        history: v.get("history").cloned().unwrap_or_default(),
        requests: v
            .get("requests")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    })
}

pub async fn poll_metrics(host: &str, port: u16, auth: &HttpAuth) -> Option<StrataMetrics> {
    parse_metrics(&http_get(host, port, "/metrics", auth).await.ok()?)
}

impl StrataMetrics {
    pub fn state(&self) -> &str {
        text(&self.live, "state").unwrap_or("unknown")
    }

    pub fn processing(&self) -> bool {
        matches!(self.state(), "reading" | "generating")
    }

    pub fn context_max(&self) -> Option<usize> {
        number(&self.engine, "max_context")
            .or_else(|| number(&self.engine, "context"))
            .map(|n| n as usize)
    }

    /// Monitor context, not the total residency of parked conversations.
    pub fn context_used(&self) -> Option<usize> {
        let (source, generated_key) = if self.state() == "idle" {
            (self.requests.first()?, "output_tokens")
        } else {
            (&self.live, "generated")
        };
        let prompt = number(source, "prompt_tokens")? as usize;
        Some(prompt.saturating_add(number(source, generated_key)? as usize))
    }

    pub fn decode_rate(&self) -> Option<f32> {
        match self.state() {
            "generating" => number(&self.live, "tok_s").map(|n| n.min(f32::MAX as f64) as f32),
            "reading" | "idle" | "unloaded" => Some(0.0),
            _ => None,
        }
    }

    pub fn prefill_rate(&self) -> Option<f32> {
        match self.state() {
            "reading" | "generating" => {
                number(&self.live, "prefill_tok_s_mean").map(|n| n.min(f32::MAX as f64) as f32)
            }
            "idle" | "unloaded" => Some(0.0),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct StrataAdapter {
    task: i64,
    prev_processing: bool,
    prev_generated: usize,
    prev_requests: Option<f64>,
}

impl StrataAdapter {
    pub fn observe(&mut self, m: StrataMetrics) -> LiveStats {
        let processing = m.processing();
        let completed = number(&m.totals, "requests");
        let generated = number(&m.live, "generated").unwrap_or(0.0) as usize;
        if processing
            && (!self.prev_processing
                || generated < self.prev_generated
                || completed != self.prev_requests)
        {
            self.task = self.task.saturating_add(1);
        }
        self.prev_processing = processing;
        self.prev_generated = generated;
        self.prev_requests = completed;
        // Match the Monitor's context display: the live request, else the last
        // completed one. This is not a count of all resident/parked KV tokens.
        let source = if m.state() == "idle" {
            m.requests.first().unwrap_or(&Value::Null)
        } else {
            &m.live
        };
        let prompt = number(source, "prompt_tokens").unwrap_or(0.0) as usize;
        LiveStats {
            ctx_max: m.context_max().unwrap_or(0),
            prompt_tokens: prompt,
            // Progress includes a reused prefix; it is NOT newly computed
            // prefill. Use Strata's measured prefill rate instead of deltas.
            prompt_processed: 0,
            decoded: if processing {
                generated
            } else {
                number(source, "output_tokens").unwrap_or(0.0) as usize
            },
            decoded_present: if processing {
                number(&m.live, "generated").is_some()
            } else {
                number(source, "output_tokens").is_some()
            },
            cache_tokens: if processing {
                0
            } else {
                number(source, "reused").unwrap_or(0.0) as usize
            },
            cache_unknown: processing || number(source, "reused").is_none(),
            processing,
            n_slots: 1,
            slots_busy: usize::from(processing),
            spec_types: if number(&m.engine, "spec").is_some_and(|n| n > 0.0) {
                if number(&m.engine, "mtp_max").is_some_and(|n| n > 0.0) {
                    "mtp".into()
                } else {
                    "lookup".into()
                }
            } else {
                "none".into()
            },
            spec_depth: number(&m.engine, "mtp_max").unwrap_or(0.0) as usize,
            id_task: self.task,
            kv_tokens: m.context_used(),
            strata: Some(m),
            ..Default::default()
        }
    }
}

/// The native `strata --serve` child. It holds the GPU memory but speaks
/// only to its parent over a pipe, so it is folded into the server.
pub fn is_engine(process_name: &str, cmdline: &str) -> bool {
    let argv0 = cmdline.split_whitespace().next().unwrap_or(process_name);
    let base = Path::new(argv0)
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    (base == "strata" || base == "strata.exe") && cmdline.split_whitespace().any(|t| t == "--serve")
}

/// What the server's `--config` JSON says about the model.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StrataConfig {
    pub model_name: Option<String>,
    /// First GGUF shard (`--native`), which carries the header.
    pub gguf: Option<PathBuf>,
    pub max_context: Option<usize>,
}

pub fn parse_config(body: &str) -> Option<StrataConfig> {
    let v: Value = serde_json::from_str(body).ok()?;
    let args: Vec<&str> = v
        .get("args")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    let flag = |name: &str| {
        args.iter().enumerate().find_map(|(i, a)| {
            a.strip_prefix(&format!("{name}="))
                .or_else(|| (*a == name).then(|| args.get(i + 1).copied()).flatten())
                .map(str::to_string)
        })
    };
    Some(StrataConfig {
        model_name: v
            .get("model_name")
            .and_then(|m| m.as_str())
            .map(str::to_string),
        gguf: flag("--native").map(PathBuf::from),
        max_context: flag("--max-context").and_then(|s| s.parse().ok()),
    })
}

/// The `--config` path on a server command line, resolved against `cwd`.
pub fn config_path(cmdline: &str, cwd: Option<&Path>) -> Option<PathBuf> {
    let p = PathBuf::from(crate::sglang::cmdline_flag(cmdline, "--config")?);
    Some(match cwd {
        Some(dir) if p.is_relative() => dir.join(p),
        _ => p,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> StrataMetrics {
        parse_metrics(&std::fs::read_to_string(format!("fixtures/strata-{name}.json")).unwrap())
            .unwrap()
    }

    #[test]
    fn recorded_idle_and_busy_mapping() {
        let mut a = StrataAdapter::default();
        let idle = fixture("idle");
        assert_eq!(
            text(&idle.engine, "model"),
            Some("Qwen3.8-Flash-Next-IQ3_S")
        );
        assert_eq!(number(&idle.engine, "expert_slots"), Some(6958.0));
        let s = a.observe(idle);
        assert!(!s.processing);
        assert_eq!(s.ctx_max, 262144);
        assert_eq!(s.ctx_used(), 92614 + 16);
        assert_eq!(s.cache_tokens, 92573);
        let busy = fixture("busy");
        assert_eq!(busy.decode_rate(), Some(46.0));
        let s = a.observe(busy);
        assert!(s.processing);
        assert_eq!(s.decoded, 215);
        assert_eq!(s.ctx_used(), 93489 + 215);
        assert!(s.cache_unknown); // live reuse isn't exposed
        assert_eq!(s.prompt_processed, 0);
    }

    #[test]
    fn recorded_prefill_and_request_boundaries() {
        let mut a = StrataAdapter::default();
        let busy = a.observe(fixture("busy"));
        let pre = a.observe(fixture("prefill"));
        assert!(pre.processing);
        assert!(pre.id_task > busy.id_task);
        assert_eq!(pre.decoded, 0);
        assert_eq!(pre.strata.unwrap().state(), "reading");
        let idle = a.observe(fixture("idle"));
        assert!(!idle.processing);
    }

    #[test]
    fn rates_use_server_window_not_prompt_progress_or_poll_time() {
        use crate::perf::{PerfTracker, Phase};
        use std::time::{Duration, Instant};
        let mut a = StrataAdapter::default();
        let mut p = PerfTracker::new();
        let now = Instant::now();
        p.observe(&a.observe(fixture("idle")), now);
        p.observe(
            &a.observe(fixture("busy")),
            now + Duration::from_millis(200),
        );
        assert_eq!(p.phase, Phase::Decode);
        assert_eq!(p.decode_tps, 46.0);
        assert_eq!(p.prefill_tps, 208.4);
        assert_eq!(p.session_requests, 17);
        assert!(p.current.is_none());
        assert!(p.history.is_empty()); // do not invent TTFT/request records
        p.observe(
            &a.observe(fixture("prefill")),
            now + Duration::from_millis(400),
        );
        assert_eq!(p.phase, Phase::Prefill);
        assert_eq!(p.decode_tps, 0.0);
    }

    #[test]
    fn missing_renamed_and_wrong_types_are_unknown() {
        let m = parse_metrics(r#"{"engine":{"kv":"int8","context":8192},"live":{"state":"generating","new_rate":45,"generated":"oops"}}"#).unwrap();
        assert_eq!(m.context_max(), Some(8192));
        assert!(m.decode_rate().is_none());
        assert!(m.prefill_rate().is_none());
        let s = StrataAdapter::default().observe(m);
        assert!(!s.decoded_present);
        assert!(s.cache_unknown);
        assert_eq!(s.ctx_used(), 0);
        assert!(s.strata.as_ref().unwrap().context_used().is_none());
        assert!(number(&serde_json::json!({"n":-1}), "n").is_none());
    }

    #[test]
    fn malformed_error_and_other_servers_are_rejected() {
        assert!(parse_metrics(include_str!("../fixtures/strata-error.json")).is_none());
        for body in [
            "",
            "not JSON",
            "[]",
            "{}",
            r#"{"error":{"message":"engine unavailable"}}"#,
            r#"{"engine":{},"live":{}}"#,
            r#"{"data":[{"id":"strata"}]}"#,
        ] {
            assert!(parse_metrics(body).is_none(), "{body}");
        }
    }

    #[test]
    fn acceptance_optional_and_windowed_across_restarts() {
        use crate::perf::PerfTracker;
        use std::time::{Duration, Instant};
        let modern = parse_metrics(include_str!("../fixtures/strata-metrics-0.1.35.json")).unwrap();
        assert!((draft_acceptance(&modern.requests[0]).unwrap() - 641.0 / 728.0).abs() < 1e-6);
        assert!(draft_acceptance(&modern.requests[2]).is_none());
        assert_eq!(draft_counts(&modern.requests[3]), Some((0, 0)));
        assert!(draft_acceptance(&modern.requests[3]).is_none());
        for v in [
            serde_json::json!({"drafts_offered": 1, "drafts_accepted": 2}),
            serde_json::json!({"drafts_offered": null, "drafts_accepted": 0}),
            serde_json::json!({"drafts_offered": -1, "drafts_accepted": 0}),
        ] {
            assert!(draft_counts(&v).is_none());
        }
        let mut baseline = modern.clone();
        baseline.totals = serde_json::json!({"since":1000,"requests":1,"output_tokens":100,"drafts_offered":100,"drafts_accepted":88});
        let mut a = StrataAdapter::default();
        let mut p = PerfTracker::new();
        let now = Instant::now();
        p.observe(&a.observe(baseline), now);
        assert!(p.spec.available);
        assert_eq!(p.spec.drafts_per_sec, 0.0); // initial sums aren't window deltas
        p.observe(&a.observe(modern.clone()), now + Duration::from_millis(400));
        assert!((p.spec.accept_rate - 641.0 / 728.0).abs() < 1e-6);
        assert_eq!(p.spec.totals.accepted, 729);
        assert_eq!(p.spec.totals.verify_steps, 0); // not inferable
        p.observe(&a.observe(modern.clone()), now + Duration::from_secs(3));
        assert_eq!(p.spec.drafts_per_sec, 0.0); // quiet window expires

        let mut reset = modern.clone();
        reset.totals["drafts_offered"] = 0.into();
        reset.totals["drafts_accepted"] = 0.into();
        p.observe(&a.observe(reset.clone()), now + Duration::from_secs(4));
        assert_eq!(p.spec.accept_rate, 0.0);
        assert!(p.spec.accept_hist.is_empty());
        reset.totals["drafts_offered"] = 10.into();
        reset.totals["drafts_accepted"] = 5.into();
        p.observe(&a.observe(reset), now + Duration::from_millis(4400));
        assert_eq!(p.spec.accept_rate, 0.5);
        // A changed server start time resets even if sums increased across
        // the restart (counter decrease alone doesn't detect that case).
        let mut restarted = modern.clone();
        restarted.totals["since"] = 2000.into();
        p.observe(&a.observe(restarted), now + Duration::from_secs(5));
        assert_eq!(p.spec.drafts_per_sec, 0.0);
        assert!(p.spec.accept_hist.is_empty());
        let mut null = modern;
        null.totals["drafts_offered"] = Value::Null;
        p.observe(&a.observe(null), now + Duration::from_secs(6));
        assert!(!p.spec.available);
        p.observe(&a.observe(fixture("idle")), now + Duration::from_secs(7));
        assert!(!p.spec.available); // 0.1.31 has no fields
    }

    #[test]
    fn upstream_older_shape_keeps_acceptance_unknown() {
        let m = parse_metrics(include_str!("../fixtures/strata-metrics-upstream.json")).unwrap();
        assert_eq!(m.context_max(), Some(32768));
        assert_eq!(m.context_used(), Some(95 + 16371));
        assert_eq!(m.decode_rate(), Some(43.9));
        assert!(m.prefill_rate().is_none());
        assert!(draft_counts(&m.totals).is_none());
        assert!(draft_acceptance(&m.requests[0]).is_none());
    }

    #[test]
    fn native_engine_and_config_facts() {
        assert!(is_engine(
            "strata",
            "/opt/strata --serve --native /models/a.gguf"
        ));
        assert!(!is_engine("strata", "/opt/strata --bench"));
        assert!(!is_engine(
            "python",
            "python serve/server.py --engine strata"
        ));
        let cfg = parse_config(
            r#"{"model_name":"qwen","args":["--native=/models/a.gguf","--max-context","32768"]}"#,
        )
        .unwrap();
        assert_eq!(cfg.gguf, Some(PathBuf::from("/models/a.gguf")));
        assert_eq!(cfg.max_context, Some(32768));
        assert_eq!(cfg.model_name.as_deref(), Some("qwen"));
        assert_eq!(
            config_path(
                "python serve/server.py --config=a.json",
                Some(Path::new("/srv"))
            ),
            Some(PathBuf::from("/srv/a.json"))
        );
    }

    #[tokio::test]
    async fn unreachable_returns_none() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        assert!(poll_metrics("127.0.0.1", port, &HttpAuth::default())
            .await
            .is_none());
    }
}
