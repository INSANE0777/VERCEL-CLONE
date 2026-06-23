use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::edge::middleware::MiddlewareType;

// ── Database Models ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: uuid::Uuid,
    pub name: String,
    pub github_repo_full_name: String,
    pub github_repo_url: String,
    pub production_branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Deployment {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub sha: String,
    pub branch: String,
    pub status: String,
    pub build_logs: Option<String>,
    pub framework: Option<String>,
    pub url: Option<String>,
    pub is_production: bool,
    pub github_comment_id: Option<i64>,
    pub github_pr_number: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnvVar {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub key: String,
    pub value: String,
    pub environment: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BuildCache {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub cache_key: String,
    pub storage_path: String,
    pub size_bytes: i64,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Domain {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub domain: String,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MiddlewareRuleDb {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub rule_type: String,
    pub pattern: String,
    pub target: String,
    pub status_code: Option<i32>,
    pub header_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl MiddlewareRuleDb {
    pub fn to_rule(&self) -> crate::edge::middleware::MiddlewareRule {
        crate::edge::middleware::MiddlewareRule {
            id: self.id,
            project_id: self.project_id,
            rule_type: MiddlewareType::from_db(&self.rule_type),
            pattern: self.pattern.clone(),
            target: self.target.clone(),
            status_code: self.status_code,
            header_name: self.header_name.clone(),
        }
    }
}

// ── API Request/Response Types ────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub github_repo_full_name: String,
    pub production_branch: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ManualDeployRequest {
    pub branch: Option<String>,
    pub sha: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetEnvVarRequest {
    pub key: String,
    pub value: String,
    pub environment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddDomainRequest {
    pub domain: String,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub github_repo_full_name: String,
    pub production_branch: String,
    pub latest_deployment: Option<DeploymentResponse>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub active_builds: usize,
    pub queue_depth: u64,
}

// ── NATS Message Types ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildJob {
    pub deployment_id: String,
    pub project_id: String,
    pub sha: String,
    pub branch: String,
    pub repo_full_name: String,
    pub repo_url: String,
    pub is_production: bool,
    pub env_vars: Vec<(String, String)>,
    pub attempt: u32,
}

// ── Deployment status enum ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BuildStatus {
    Queued,
    Building,
    Ready,
    Error,
    Cancelled,
}

impl std::fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildStatus::Queued => write!(f, "queued"),
            BuildStatus::Building => write!(f, "building"),
            BuildStatus::Ready => write!(f, "ready"),
            BuildStatus::Error => write!(f, "error"),
            BuildStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}
