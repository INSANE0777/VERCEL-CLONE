use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ── API Types ──────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub active_builds: usize,
    pub queue_depth: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeploymentResponse {
    pub id: String,
    pub status: String,
    pub url: Option<String>,
    pub branch: String,
    pub sha: String,
    pub is_production: bool,
    pub framework: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub github_repo_full_name: String,
    pub production_branch: String,
    pub latest_deployment: Option<DeploymentResponse>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub github_repo_full_name: String,
    pub production_branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FrameworkCount {
    pub framework: String,
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DayCount {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalyticsSummary {
    pub total_projects: i64,
    pub total_deployments: i64,
    #[serde(default)]
    pub total_builds: i64,
    #[serde(default)]
    pub successful_builds: i64,
    #[serde(default)]
    pub failed_builds: i64,
    pub success_rate: f64,
    pub avg_build_duration_secs: f64,
    #[serde(default)]
    pub frameworks: Vec<FrameworkCount>,
    #[serde(default)]
    pub deploys_last_7_days: Vec<DayCount>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectAnalytics {
    pub total_deployments: i64,
    pub ready: i64,
    pub errors: i64,
    pub avg_build_duration_secs: f64,
    #[serde(default)]
    pub recent_deployments: Vec<RecentDeployment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecentDeployment {
    pub id: String,
    pub status: String,
    pub branch: String,
    pub framework: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogResponse {
    pub logs: String,
}

// ── API Client ─────────────────────────────────────────────

pub fn api_base() -> String {
    crate::app::use_app_state().api_base.clone()
}

pub async fn api_get<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let url = format!("{}{}", api_base(), path);
    reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json::<T>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn api_post<T: serde::de::DeserializeOwned, B: serde::Serialize>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    let url = format!("{}{}", api_base(), path);
    let client = reqwest::Client::new();
    client
        .post(&url)
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<T>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn api_delete(path: &str) -> Result<(), String> {
    let url = format!("{}{}", api_base(), path);
    let client = reqwest::Client::new();
    client
        .delete(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Shared UI Components ───────────────────────────────────

#[component]
pub fn Badge(status: String) -> impl IntoView {
    let class = match status.as_str() {
        "ready" => "badge badge-ready",
        "error" => "badge badge-error",
        "building" => "badge badge-building",
        _ => "badge badge-queued",
    };
    view! {
        <span class=class>{status}</span>
    }
}

#[component]
pub fn StatCard(label: String, value: String, accent: bool) -> impl IntoView {
    let class = if accent { "stat-card accent" } else { "stat-card" };
    view! {
        <div class=class>
            <div class="stat-value">{value}</div>
            <div class="stat-label">{label}</div>
        </div>
    }
}

pub fn time_ago(iso: &str) -> String {
    // Try parsing; if it fails, return the raw string
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return iso.to_string();
    };
    let now = chrono::Utc::now();
    let dur = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
    let secs = dur.num_seconds();
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

pub fn short_sha(sha: &str) -> String {
    if sha.len() >= 7 {
        sha[..7].to_string()
    } else {
        sha.to_string()
    }
}
