mod handlers;

use crate::AppState;
use axum::{
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

/// API key auth middleware. If api_key is empty, auth is disabled (dev mode).
async fn auth_middleware(
    State(api_key): State<String>,
    request: Request,
    next: Next,
) -> Response {
    if api_key.is_empty() {
        return next.run(request).await;
    }
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());
    match auth_header {
        Some(h) if h.starts_with("Bearer ") && h[7..] == api_key => next.run(request).await,
        _ => (StatusCode::UNAUTHORIZED, "Unauthorized").into_response(),
    }
}

pub fn create_router(state: Arc<AppState>) -> Router {
    let api_key = state.config.api_key.clone();

    // Protected routes — require Bearer token if API_KEY is set
    let protected = Router::new()
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
        .layer(from_fn_with_state(api_key, auth_middleware));

    // Public routes — no auth required
    let public = Router::new()
        .route("/api/health", get(handlers::health))
        .route("/webhooks/github", post(handlers::github_webhook))
        .route("/", get(handlers::dashboard))
        .route("/dashboard", get(handlers::dashboard));

    // CORS — restrictive in production, permissive for localhost dev
    let cors = if state.config.base_domain == "localhost" {
        CorsLayer::permissive()
    } else {
        let base_origin: HeaderValue =
            format!("https://{}", state.config.base_domain)
                .parse()
                .expect("Invalid base domain for CORS");
        CorsLayer::new()
            .allow_origin(AllowOrigin::list([
                base_origin,
                "http://localhost:3000".parse().unwrap(),
            ]))
            .allow_methods(AllowMethods::list([
                Method::GET,
                Method::POST,
                Method::DELETE,
                Method::OPTIONS,
            ]))
            .allow_headers(AllowHeaders::list([
                "Authorization".parse().unwrap(),
                "Content-Type".parse().unwrap(),
            ]))
    };

    Router::new()
        .merge(protected)
        .merge(public)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
