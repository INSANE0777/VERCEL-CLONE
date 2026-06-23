mod handlers;

use crate::AppState;
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(handlers::health))
        .route("/api/projects", get(handlers::list_projects).post(handlers::create_project))
        .route("/api/projects/:id", get(handlers::get_project).delete(handlers::delete_project))
        .route("/api/projects/:id/deployments", get(handlers::list_deployments))
        .route("/api/projects/:id/deploy", post(handlers::trigger_deploy))
        .route("/api/projects/:id/env", get(handlers::list_env_vars).post(handlers::set_env_var))
        .route("/api/projects/:id/env/:key", delete(handlers::delete_env_var))
        .route("/api/projects/:id/domains", get(handlers::list_domains).post(handlers::add_domain))
        .route("/api/projects/:id/domains/:domain", delete(handlers::delete_domain))
        .route("/api/projects/:id/middleware", get(handlers::list_middleware_rules).post(handlers::create_middleware_rule))
        .route("/api/projects/:id/middleware/:rule_id", delete(handlers::delete_middleware_rule))
        .route("/api/projects/:id/analytics", get(handlers::project_analytics))
        .route("/api/analytics/summary", get(handlers::analytics_summary))
        .route("/api/deployments/:id", get(handlers::get_deployment))
        .route("/api/deployments/:id/logs", get(handlers::get_deployment_logs))
        .route("/api/deployments/:id/status/stream", get(handlers::stream_deployment_status))
        .route("/webhooks/github", post(handlers::github_webhook))
        .route("/", get(handlers::dashboard))
        .route("/dashboard", get(handlers::dashboard))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
