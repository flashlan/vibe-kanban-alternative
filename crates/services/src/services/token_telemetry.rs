use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One agent's accumulated token usage within a single day.
#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
pub struct TokenTelemetryAgent {
    pub agent: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
}

/// One day's aggregated LLM token usage, for the Settings → Usage dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
pub struct TokenTelemetryDay {
    pub day: String,
    pub agents: Vec<TokenTelemetryAgent>,
    pub total_input: i64,
    pub total_output: i64,
    pub total_cache_read: i64,
    pub total_cache_creation: i64,
}

/// Summary of LLM token + KV-cache telemetry across all agents (last 30
/// days, in-memory only — resets on server restart, same as
/// [`super::mem0_relevance::Mem0RelevanceSummary`]).
#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
pub struct TokenTelemetrySummary {
    /// Last 30 days, oldest first.
    pub days: Vec<TokenTelemetryDay>,
    pub total_input: i64,
    pub total_output: i64,
    pub total_cache_read: i64,
    pub total_cache_creation: i64,
    /// `cache_read / (input + cache_read + cache_creation)`, if denominator > 0.
    pub cache_hit_pct: Option<f64>,
}

/// Internal accumulator for one (day, agent) bucket.
#[derive(Debug, Default)]
struct AgentDayAccum {
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
}

/// In-memory day-bucketed ledger of per-agent LLM token usage, reported via
/// `POST /api/usage/token-telemetry`. Deliberately in-memory only (like
/// [`super::mem0_relevance::Mem0RelevanceService`]): an observability aid,
/// not data that needs to survive a server restart.
#[derive(Clone)]
pub struct TokenTelemetryService {
    /// Key: `"YYYY-MM-DD|agent_name"`
    buckets: std::sync::Arc<DashMap<String, AgentDayAccum>>,
}

impl TokenTelemetryService {
    pub fn new() -> Self {
        Self {
            buckets: std::sync::Arc::new(DashMap::new()),
        }
    }

    /// Record one batch of token usage for the given agent. Called when an
    /// execution emits a `TokenUsageInfo` entry or when an execution
    /// completes. Idempotency is the caller's responsibility — this method
    /// simply accumulates.
    pub fn record(
        &self,
        agent: &str,
        input_tokens: u32,
        output_tokens: u32,
        cache_read_tokens: u32,
        cache_creation_tokens: u32,
    ) {
        let day = Utc::now().format("%Y-%m-%d").to_string();
        let key = format!("{day}|{agent}");
        let mut entry = self.buckets.entry(key).or_default();
        entry.input_tokens += input_tokens as i64;
        entry.output_tokens += output_tokens as i64;
        entry.cache_read_tokens += cache_read_tokens as i64;
        entry.cache_creation_tokens += cache_creation_tokens as i64;
    }

    /// Last 30 days of per-agent token usage, oldest first, plus totals.
    pub fn summary(&self) -> TokenTelemetrySummary {
        let cutoff = (Utc::now() - chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();

        // Collect (day, agent) buckets within the 30-day window.
        let mut day_map: std::collections::BTreeMap<String, Vec<TokenTelemetryAgent>> =
            std::collections::BTreeMap::new();

        for entry in self.buckets.iter() {
            let key = entry.key();
            let (day, agent) = match key.split_once('|') {
                Some(pair) => pair,
                None => continue,
            };
            if day < cutoff.as_str() {
                continue;
            }
            let a = entry.value();
            day_map
                .entry(day.to_string())
                .or_default()
                .push(TokenTelemetryAgent {
                    agent: agent.to_string(),
                    input_tokens: a.input_tokens,
                    output_tokens: a.output_tokens,
                    cache_read_tokens: a.cache_read_tokens,
                    cache_creation_tokens: a.cache_creation_tokens,
                });
        }

        let mut total_input: i64 = 0;
        let mut total_output: i64 = 0;
        let mut total_cache_read: i64 = 0;
        let mut total_cache_creation: i64 = 0;

        let days: Vec<TokenTelemetryDay> = day_map
            .into_iter()
            .map(|(day, agents)| {
                let di: i64 = agents.iter().map(|a| a.input_tokens).sum();
                let do_: i64 = agents.iter().map(|a| a.output_tokens).sum();
                let dcr: i64 = agents.iter().map(|a| a.cache_read_tokens).sum();
                let dcc: i64 = agents.iter().map(|a| a.cache_creation_tokens).sum();
                total_input += di;
                total_output += do_;
                total_cache_read += dcr;
                total_cache_creation += dcc;
                TokenTelemetryDay {
                    day,
                    agents,
                    total_input: di,
                    total_output: do_,
                    total_cache_read: dcr,
                    total_cache_creation: dcc,
                }
            })
            .collect();

        let denom = total_input + total_cache_read + total_cache_creation;
        let cache_hit_pct = if denom > 0 {
            Some(total_cache_read as f64 / denom as f64)
        } else {
            None
        };

        TokenTelemetrySummary {
            days,
            total_input,
            total_output,
            total_cache_read,
            total_cache_creation,
            cache_hit_pct,
        }
    }
}

impl Default for TokenTelemetryService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_accumulates_per_agent_into_today() {
        let svc = TokenTelemetryService::new();
        svc.record("claude", 1000, 200, 800, 100);
        svc.record("claude", 500, 100, 400, 50);
        svc.record("antigravity", 2000, 300, 1500, 0);

        let summary = svc.summary();
        assert_eq!(summary.days.len(), 1);
        assert_eq!(summary.total_input, 3500);
        assert_eq!(summary.total_output, 600);
        assert_eq!(summary.total_cache_read, 2700);
        assert_eq!(summary.total_cache_creation, 150);

        // cache_hit_pct = 2700 / (3500 + 2700 + 150) = 2700 / 6350 ≈ 0.425
        let pct = summary.cache_hit_pct.unwrap();
        assert!((pct - 2700.0 / 6350.0).abs() < 1e-9);

        let day = &summary.days[0];
        assert_eq!(day.agents.len(), 2);
    }

    #[test]
    fn summary_with_no_records_is_empty() {
        let svc = TokenTelemetryService::new();
        let summary = svc.summary();
        assert!(summary.days.is_empty());
        assert_eq!(summary.total_input, 0);
        assert!(summary.cache_hit_pct.is_none());
    }

    #[test]
    fn old_day_is_excluded() {
        let svc = TokenTelemetryService::new();
        svc.record("claude", 100, 50, 80, 10);
        // Manually insert an old bucket.
        svc.buckets.insert(
            "2000-01-01|claude".to_string(),
            AgentDayAccum {
                input_tokens: 9999,
                output_tokens: 9999,
                cache_read_tokens: 9999,
                cache_creation_tokens: 9999,
            },
        );

        let summary = svc.summary();
        assert_eq!(summary.days.len(), 1, "the old day must be filtered out");
        assert_eq!(summary.total_input, 100);
    }
}
