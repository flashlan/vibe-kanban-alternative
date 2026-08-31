//! Process-local snapshots of provider quota information.
//!
//! Provider limits are account state, not conversation state. Keeping the
//! latest snapshot here lets executors publish it without coupling the CLI
//! protocol to the server API. The server exposes the snapshot as best-effort
//! data; absence means that the provider did not make a safe machine-readable
//! value available.

use std::sync::{LazyLock, RwLock};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProviderQuotaWindow {
    pub name: String,
    pub used_percent: Option<f64>,
    pub limit_value: Option<f64>,
    pub used_value: Option<f64>,
    pub unit: Option<String>,
    pub duration_minutes: Option<i64>,
    pub resets_at: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProviderQuotaSnapshot {
    pub provider: String,
    pub plan: Option<String>,
    pub windows: Vec<ProviderQuotaWindow>,
    pub credits_balance: Option<String>,
    pub credits_unlimited: bool,
    pub status: Option<String>,
    pub observed_at: i64,
}

static SNAPSHOTS: LazyLock<RwLock<Vec<ProviderQuotaSnapshot>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

pub fn record_snapshot(mut snapshot: ProviderQuotaSnapshot) {
    snapshot.observed_at = chrono::Utc::now().timestamp();
    let mut snapshots = SNAPSHOTS.write().expect("provider quota lock poisoned");
    if let Some(current) = snapshots
        .iter_mut()
        .find(|current| current.provider == snapshot.provider)
    {
        *current = snapshot;
    } else {
        snapshots.push(snapshot);
    }
}

/// Merge a single event window without discarding other windows from the same
/// provider. Claude's stream reports one rate-limit bucket per event.
pub fn record_window(
    provider: &str,
    plan: Option<String>,
    window: ProviderQuotaWindow,
    status: Option<String>,
) {
    let mut snapshots = SNAPSHOTS.write().expect("provider quota lock poisoned");
    let snapshot = snapshots
        .iter_mut()
        .find(|snapshot| snapshot.provider == provider);
    if let Some(snapshot) = snapshot {
        if let Some(existing) = snapshot.windows.iter_mut().find(|w| w.name == window.name) {
            *existing = window;
        } else {
            snapshot.windows.push(window);
        }
        if plan.is_some() {
            snapshot.plan = plan;
        }
        snapshot.status = status;
        snapshot.observed_at = chrono::Utc::now().timestamp();
    } else {
        snapshots.push(ProviderQuotaSnapshot {
            provider: provider.to_string(),
            plan,
            windows: vec![window],
            credits_balance: None,
            credits_unlimited: false,
            status,
            observed_at: chrono::Utc::now().timestamp(),
        });
    }
}

pub fn snapshots() -> Vec<ProviderQuotaSnapshot> {
    SNAPSHOTS
        .read()
        .expect("provider quota lock poisoned")
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(name: &str, used_percent: f64) -> ProviderQuotaWindow {
        ProviderQuotaWindow {
            name: name.to_string(),
            used_percent: Some(used_percent),
            limit_value: None,
            used_value: None,
            unit: None,
            duration_minutes: None,
            resets_at: Some(100),
            status: Some("allowed".to_string()),
        }
    }

    #[test]
    fn windows_from_separate_events_are_merged() {
        record_window(
            "test-provider",
            Some("test".to_string()),
            window("five_hour", 20.0),
            None,
        );
        record_window("test-provider", None, window("seven_day", 40.0), None);

        let snapshot = snapshots()
            .into_iter()
            .find(|snapshot| snapshot.provider == "test-provider")
            .unwrap();
        assert_eq!(snapshot.windows.len(), 2);
        assert_eq!(snapshot.plan.as_deref(), Some("test"));
    }
}
