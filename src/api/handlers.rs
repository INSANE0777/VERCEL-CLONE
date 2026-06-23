use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use futures_util::StreamExt;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;

use crate::models::*;
use crate::AppState;

// ── Health ────────────────────────────────────────────────────

pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime = std::time::SystemTime::now()
        .duration_since(state.start_time)
        .unwrap_or_default()
        .as_secs();

    let queue_depth = state.queue.pending_count().await.unwrap_or(0);

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: uptime,
        active_builds: state.active_builds.load(std::sync::atomic::Ordering::Relaxed),
        queue_depth,
    })
}

// ── Projects ──────────────────────────────────────────────────

pub async fn list_projects(State(state): State<Arc<AppState>>) -> Result<Json<Vec<ProjectResponse>>, (StatusCode, String)> {
    let projects = state.db.list_projects().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut responses = Vec::with_capacity(projects.len());
    for p in projects {
        let latest = state.db.get_latest_deployment(p.id).await.ok().flatten();
        responses.push(ProjectResponse {
            id: p.id.to_string(),
            name: p.name,
            github_repo_full_name: p.github_repo_full_name,
            production_branch: p.production_branch,
            latest_deployment: latest.map(deployment_to_response),
            created_at: p.created_at.to_rfc3339(),
        });
    }
    Ok(Json(responses))
}

pub async fn get_project(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<ProjectResponse>, (StatusCode, String)> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let p = state.db.get_project(uuid).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let latest = state.db.get_latest_deployment(p.id).await.ok().flatten();
    Ok(Json(ProjectResponse {
        id: p.id.to_string(), name: p.name,
        github_repo_full_name: p.github_repo_full_name,
        production_branch: p.production_branch,
        latest_deployment: latest.map(deployment_to_response),
        created_at: p.created_at.to_rfc3339(),
    }))
}

pub async fn create_project(State(state): State<Arc<AppState>>, Json(req): Json<CreateProjectRequest>) -> Result<(StatusCode, Json<ProjectResponse>), (StatusCode, String)> {
    let github_url = format!("https://github.com/{}", req.github_repo_full_name);
    let production_branch = req.production_branch.unwrap_or_else(|| "main".into());
    let p = state.db.create_project(&req.name, &req.github_repo_full_name, &github_url, &production_branch)
        .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok((StatusCode::CREATED, Json(ProjectResponse {
        id: p.id.to_string(), name: p.name,
        github_repo_full_name: p.github_repo_full_name,
        production_branch: p.production_branch,
        latest_deployment: None, created_at: p.created_at.to_rfc3339(),
    })))
}

// ── Deployments ───────────────────────────────────────────────

pub async fn list_deployments(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Vec<DeploymentResponse>>, (StatusCode, String)> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let deployments = state.db.list_deployments(uuid).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(deployments.into_iter().map(deployment_to_response).collect()))
}

pub async fn get_deployment(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<DeploymentResponse>, (StatusCode, String)> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let d = state.db.get_deployment(uuid).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(deployment_to_response(d)))
}

pub async fn get_deployment_logs(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let d = state.db.get_deployment(uuid).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(serde_json::json!({ "id": d.id.to_string(), "status": d.status, "logs": d.build_logs.unwrap_or_default() })))
}

pub async fn trigger_deploy(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<ManualDeployRequest>) -> Result<(StatusCode, Json<DeploymentResponse>), (StatusCode, String)> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid project UUID".into()))?;
    let project = state.db.get_project(project_id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    let branch = req.branch.unwrap_or_else(|| project.production_branch.clone());
    let sha = req.sha.unwrap_or_else(|| "HEAD".into());
    let is_production = branch == project.production_branch;

    let url = if is_production {
        format!("{}.localhost", project.name)
    } else {
        format!("{}-{:.8}.localhost", project.name, uuid::Uuid::new_v4())
    };

    let d = state.db.create_deployment(project_id, &sha, &branch, is_production, &url)
        .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get env vars for this deployment
    let env_vars = state.db.get_env_vars(project_id, if is_production { "production" } else { "preview" }).await
        .unwrap_or_default();

    let job = BuildJob {
        deployment_id: d.id.to_string(),
        project_id: project_id.to_string(),
        sha: sha.clone(), branch: branch.clone(),
        repo_full_name: project.github_repo_full_name.clone(),
        repo_url: project.github_repo_url.clone(),
        is_production,
        env_vars: env_vars.into_iter().map(|e| (e.key, e.value)).collect(),
        attempt: 1,
    };

    state.queue.enqueue(&job).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(deployment_to_response(d))))
}

// ── Environment Variables ─────────────────────────────────────

pub async fn list_env_vars(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Vec<EnvVar>>, (StatusCode, String)> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let vars = state.db.get_env_vars(project_id, "production").await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(vars))
}

pub async fn set_env_var(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<SetEnvVarRequest>) -> Result<Json<EnvVar>, (StatusCode, String)> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let env = state.db.set_env_var(project_id, &req.key, &req.value, &req.environment.unwrap_or_else(|| "production".into()))
        .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(env))
}

pub async fn delete_env_var(State(state): State<Arc<AppState>>, Path((id, key)): Path<(String, String)>) -> Result<StatusCode, (StatusCode, String)> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    state.db.delete_env_var(project_id, &key, "production").await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Domains ───────────────────────────────────────────────────

pub async fn list_domains(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Vec<Domain>>, (StatusCode, String)> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let domains = state.db.list_domains(project_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(domains))
}

pub async fn add_domain(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<AddDomainRequest>) -> Result<Json<Domain>, (StatusCode, String)> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let domain = state.db.add_domain(project_id, &req.domain)
        .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(domain))
}

pub async fn delete_domain(State(state): State<Arc<AppState>>, Path((id, domain)): Path<(String, String)>) -> Result<StatusCode, (StatusCode, String)> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    state.db.delete_domain(project_id, &domain).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Middleware Rules ──────────────────────────────────────────

pub async fn list_middleware_rules(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, String)> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let rules = state.db.list_middleware_rules(project_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let result: Vec<serde_json::Value> = rules.into_iter().map(|r| serde_json::json!({
        "id": r.id.to_string(),
        "project_id": r.project_id.to_string(),
        "rule_type": r.rule_type,
        "pattern": r.pattern,
        "target": r.target,
        "status_code": r.status_code,
        "header_name": r.header_name,
        "created_at": r.created_at.to_rfc3339(),
    })).collect();
    Ok(Json(result))
}

pub async fn create_middleware_rule(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<crate::edge::middleware::CreateMiddlewareRuleRequest>) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let rule = state.db.create_middleware_rule(
        project_id,
        req.rule_type.as_str(),
        &req.pattern,
        &req.target,
        req.status_code,
        req.header_name.as_deref(),
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "id": rule.id.to_string(),
        "project_id": rule.project_id.to_string(),
        "rule_type": rule.rule_type,
        "pattern": rule.pattern,
        "target": rule.target,
        "status_code": rule.status_code,
        "header_name": rule.header_name,
        "created_at": rule.created_at.to_rfc3339(),
    }))))
}

pub async fn delete_middleware_rule(State(state): State<Arc<AppState>>, Path((id, rule_id)): Path<(String, String)>) -> Result<StatusCode, (StatusCode, String)> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    let rule_uuid = uuid::Uuid::parse_str(&rule_id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid rule UUID".into()))?;
    state.db.delete_middleware_rule(project_id, rule_uuid).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_project(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".into()))?;
    state.db.delete_project(uuid).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// ── WebSocket: Deployment Status Stream ───────────────────────

pub async fn stream_deployment_status(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket| async move {
        let deployment_id = id.clone();

        // Send current deployment status immediately (no history replay gap)
        if let Ok(uuid) = uuid::Uuid::parse_str(&deployment_id) {
            if let Ok(d) = state.db.get_deployment(uuid).await {
                let status_msg = serde_json::json!({
                    "deployment_id": d.id.to_string(),
                    "status": d.status,
                    "message": "current status",
                    "timestamp": d.updated_at.to_rfc3339(),
                });
                let _ = socket.send(axum::extract::ws::Message::Text(status_msg.to_string())).await;
            }
        }

        let mut sub = match state.queue.subscribe_status().await {
            Ok(s) => s,
            Err(_) => return,
        };

        while let Some(msg) = sub.next().await {
            let payload = String::from_utf8_lossy(&msg.payload);
            if let Ok(update) = serde_json::from_str::<serde_json::Value>(&payload) {
                if update.get("deployment_id").and_then(|v| v.as_str()) == Some(&deployment_id) {
                    let _ = socket.send(axum::extract::ws::Message::Text(payload.to_string())).await;
                }
            }
        }
    })
}

// ── GitHub Webhook ────────────────────────────────────────────

pub async fn github_webhook(State(state): State<Arc<AppState>>, headers: HeaderMap, body: String) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Verify signature (mandatory — reject if header missing)
    let signature = headers.get("x-hub-signature-256")
        .ok_or((StatusCode::UNAUTHORIZED, "Missing x-hub-signature-256 header".into()))?
        .to_str().unwrap_or("");
    let secret = state.config.github_webhook_secret.as_bytes();
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "HMAC error".into()))?;
    mac.update(body.as_bytes());
    let expected = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
    if signature != expected {
        return Err((StatusCode::UNAUTHORIZED, "Invalid signature".into()));
    }

    let event = headers.get("x-github-event").and_then(|v| v.to_str().ok()).unwrap_or("");
    let payload: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid JSON".into()))?;

    match event {
        "push" => handle_push(&state, &payload).await,
        "pull_request" => handle_pull_request(&state, &payload).await,
        "ping" => Ok(Json(serde_json::json!({ "message": "pong" }))),
        _ => Ok(Json(serde_json::json!({ "message": "ignored" }))),
    }
}

async fn handle_push(state: &AppState, payload: &serde_json::Value) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let repo_full_name = payload.get("repository")
        .and_then(|r| r.get("full_name")).and_then(|n| n.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "Missing repository.full_name".into()))?;

    let branch = payload.get("ref").and_then(|r| r.as_str()).unwrap_or("")
        .replace("refs/heads/", "");
    let sha = payload.get("after").and_then(|a| a.as_str()).unwrap_or("HEAD");

    let project = match state.db.get_project_by_repo(repo_full_name).await {
        Ok(p) => p,
        Err(_) => return Ok(Json(serde_json::json!({ "message": "no project configured" }))),
    };

    let is_production = branch == project.production_branch;
    let url = if is_production {
        format!("{}.localhost", project.name)
    } else {
        format!("{}-{:.8}.localhost", project.name, uuid::Uuid::new_v4())
    };

    let d = state.db.create_deployment(project.id, sha, &branch, is_production, &url)
        .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get env vars
    let env_vars = state.db.get_env_vars(project.id, if is_production { "production" } else { "preview" }).await
        .unwrap_or_default();

    let job = BuildJob {
        deployment_id: d.id.to_string(),
        project_id: project.id.to_string(),
        sha: sha.into(), branch: branch.clone(),
        repo_full_name: project.github_repo_full_name.clone(),
        repo_url: project.github_repo_url.clone(),
        is_production,
        env_vars: env_vars.into_iter().map(|e| (e.key, e.value)).collect(),
        attempt: 1,
    };

    state.queue.enqueue(&job).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "deployment created",
        "deployment_id": d.id.to_string(),
        "url": d.url,
    })))
}

async fn handle_pull_request(state: &AppState, payload: &serde_json::Value) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Only handle opened, reopened, and synchronize (push to PR branch)
    let action = payload.get("action").and_then(|a| a.as_str()).unwrap_or("");
    if !matches!(action, "opened" | "reopened" | "synchronize") {
        return Ok(Json(serde_json::json!({ "message": format!("action '{}' ignored", action) })));
    }

    let repo_full_name = payload.get("repository")
        .and_then(|r| r.get("full_name")).and_then(|n| n.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "Missing repository.full_name".into()))?;

    let pr_number = payload.get("number").and_then(|n| n.as_u64())
        .ok_or((StatusCode::BAD_REQUEST, "Missing PR number".into()))?;

    let pr_data = payload.get("pull_request")
        .ok_or((StatusCode::BAD_REQUEST, "Missing pull_request object".into()))?;

    let sha = pr_data.get("head").and_then(|h| h.get("sha")).and_then(|s| s.as_str())
        .unwrap_or("HEAD");
    let branch = pr_data.get("head").and_then(|h| h.get("ref")).and_then(|r| r.as_str())
        .unwrap_or("");

    let project = match state.db.get_project_by_repo(repo_full_name).await {
        Ok(p) => p,
        Err(_) => return Ok(Json(serde_json::json!({ "message": "no project configured" }))),
    };

    // PR deployments are always preview (not production)
    let url = format!("{}-{:.8}.localhost", project.name, uuid::Uuid::new_v4());

    let d = state.db.create_deployment(project.id, sha, &branch, false, &url)
        .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Post PR comment if bot is enabled
    if let Some(bot) = &state.pr_bot {
        match bot.post_deployment_comment(repo_full_name, pr_number, &url, &d.id.to_string()).await {
 Ok(comment_id) => {
                if let Err(e) = state.db
                    .set_deployment_github_comment(d.id, comment_id as i64, pr_number as i32)
                    .await
                {
                    tracing::warn!("Failed to store PR comment ID: {}", e);
                }
            }
            Err(e) => tracing::warn!("Failed to post PR comment: {}", e),
        }
    }

    // Get env vars for preview environment
    let env_vars = state.db.get_env_vars(project.id, "preview").await.unwrap_or_default();

    let job = BuildJob {
        deployment_id: d.id.to_string(),
        project_id: project.id.to_string(),
        sha: sha.into(), branch: branch.to_string(),
        repo_full_name: project.github_repo_full_name.clone(),
        repo_url: project.github_repo_url.clone(),
        is_production: false,
        env_vars: env_vars.into_iter().map(|e| (e.key, e.value)).collect(),
        attempt: 1,
    };

    state.queue.enqueue(&job).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "preview deployment created for PR",
        "deployment_id": d.id.to_string(),
        "url": d.url,
        "pr_number": pr_number,
    })))
}

// ── Helpers ───────────────────────────────────────────────────

fn deployment_to_response(d: Deployment) -> DeploymentResponse {
    DeploymentResponse {
        id: d.id.to_string(), status: d.status,
        url: d.url, branch: d.branch, sha: d.sha,
        is_production: d.is_production, framework: d.framework,
        created_at: d.created_at.to_rfc3339(),
    }
}
