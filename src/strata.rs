//! Strata's JSON `/metrics` adapter. Unlike llama.cpp, its rate is already
//! windowed by the server. Preserve unknown fields as unknown, not real zeroes.
use crate::observe::{http_get, HttpAuth, LiveStats};
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct StrataMetrics {
    pub engine: Value,
    pub live: Value,
    pub totals: Value,
    pub hardware: Value,
    pub hardware_static: Value,
    pub requests: Vec<Value>,
}

/// Numeric fields can disappear or become null between engine versions.
/// Reject negative/non-finite values and strings rather than inventing data.
pub fn number(v: &Value, key: &str) -> Option<f64> {
    v.get(key)?.as_f64().filter(|n| n.is_finite() && *n >= 0.0)
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
            decoded: if processing { generated } else { 0 },
            decoded_present: number(&m.live, "generated").is_some(),
            cache_tokens: if processing {
                0
            } else {
                number(source, "reused").unwrap_or(0.0) as usize
            },
            cache_unknown: processing || number(source, "reused").is_none(),
            processing,
            id_task: self.task,
            kv_tokens: m.context_used(),
            strata: Some(m),
            ..Default::default()
        }
    }
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
