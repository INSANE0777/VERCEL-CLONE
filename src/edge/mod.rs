use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub mod middleware;

use middleware::MiddlewareRule;

/// Manages Caddy configuration for serving deployed sites.
/// Caddy provides automatic HTTPS via Let's Encrypt.
#[derive(Clone)]
pub struct EdgeRouter {
    config_dir: PathBuf,
    base_domain: String,
    /// True when routes were added via Caddy admin API (no reload needed)
    api_mode: Arc<AtomicBool>,
}

impl EdgeRouter {
    pub fn new(config_dir: &str, base_domain: &str) -> Self {
        let config_dir = PathBuf::from(config_dir);
        std::fs::create_dir_all(&config_dir).expect("Failed to create Caddy config directory");
        Self {
            config_dir,
            base_domain: base_domain.to_string(),
            api_mode: Arc::new(AtomicBool::new(false)),
        }
    }

    fn route_id_for_url(&self, deployment_url: &str) -> String {
        format!("route-{}", deployment_url.replace('.', "-").replace('*', "_"))
    }

    /// Try to add a route via Caddy admin API (atomic, no restart needed).
    /// Returns true on success, false to fall back to file-based approach.
    async fn try_add_route_api(
        &self,
        _deployment_id: &str,
        deployment_url: &str,
        artifacts_dir: &str,
        framework: &str,
    ) -> bool {
        let client = reqwest::Client::new();
        let route_id = self.route_id_for_url(deployment_url);

        let route = serde_json::json!({
            "@id": route_id,
            "match": [{"host": [deployment_url]}],
            "handle": [{
                "handler": "file_server",
                "root": artifacts_dir
            }],
            "terminal": true
        });

        match client
            .post("http://caddy:2019/config/apps/http/servers/srv0/routes")
            .header("Content-Type", "application/json")
            .json(&route)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(
                    "Added route via Caddy admin API: {} (framework: {})",
                    deployment_url, framework
                );
                true
            }
            Ok(resp) => {
                tracing::debug!(
                    "Caddy admin API returned {}, falling back to file-based",
                    resp.status()
                );
                false
            }
            Err(_) => {
                tracing::debug!("Caddy admin API unavailable, using file-based");
                false
            }
        }
    }

    /// Generate a Caddy config for a deployment
    pub async fn add_deployment(
        &self,
        deployment_id: &str,
        deployment_url: &str,
        artifacts_dir: &str,
        framework: &str,
        middleware_rules: &[MiddlewareRule],
    ) -> anyhow::Result<()> {
        let config = if self.base_domain == "localhost" {
            // Local development — no TLS
            format!(
                r#"# Auto-generated for deployment {deployment_id}
# Framework: {framework}
{deployment_url}:80 {{
    root * {artifacts_dir}
    
{middleware}
    # SPA fallback
    try_files {{path}} /index.html
    
    # Serve static files
    file_server
    
    # Compression
    encode gzip zstd
    
    # Cache static assets
    @static path /_next/static/* /assets/* /static/*
    header @static Cache-Control "public, max-age=31536000, immutable"
    
    # Deployment headers
    header X-Deployment-Id "{deployment_id}"
    header X-Framework "{framework}"
    
    # CORS (development)
    header Access-Control-Allow-Origin "*"
}}
"#,
                deployment_id = deployment_id,
                deployment_url = deployment_url,
                artifacts_dir = artifacts_dir,
                framework = framework,
                middleware = middleware::compile_middleware(middleware_rules),
            )
        } else {
            // Production — automatic HTTPS
            format!(
                r#"# Auto-generated for deployment {deployment_id}
# Framework: {framework}
{deployment_url}:80 {{
    root * {artifacts_dir}
    
{middleware}
    # SPA fallback
    try_files {{path}} /index.html
    
    # Serve static files
    file_server
    
    # Compression
    encode gzip zstd
    
    # Cache static assets
    @static path /_next/static/* /assets/* /static/*
    header @static Cache-Control "public, max-age=31536000, immutable"
    
    # Deployment headers
    header X-Deployment-Id "{deployment_id}"
    header X-Framework "{framework}"
    
    # Security headers
    header X-Content-Type-Options "nosniff"
    header X-Frame-Options "DENY"
    header Referrer-Policy "strict-origin-when-cross-origin"
}}
"#,
                deployment_id = deployment_id,
                deployment_url = deployment_url,
                artifacts_dir = artifacts_dir,
                framework = framework,
                middleware = middleware::compile_middleware(middleware_rules),
            )
        };

        let config_path = self.config_dir.join(format!(
            "{}.caddy",
            deployment_url.replace('.', "_").replace('*', "_wildcard")
        ));
        tokio::fs::write(&config_path, config).await?;

        // Try atomic route add via Caddy admin API (eliminates 2s restart downtime)
        if self.try_add_route_api(deployment_id, deployment_url, artifacts_dir, framework).await {
            self.api_mode.store(true, Ordering::Relaxed);
        } else {
            self.api_mode.store(false, Ordering::Relaxed);
        }

        tracing::info!(
            "Generated Caddy config: {} → {}",
            deployment_url,
            config_path.display()
        );
        Ok(())
    }

    /// Remove a deployment's Caddy config
    pub async fn remove_deployment(&self, deployment_url: &str) -> anyhow::Result<()> {
        // Try removing via Caddy admin API first
        let client = reqwest::Client::new();
        let route_id = self.route_id_for_url(deployment_url);
        let _ = client
            .delete(format!("http://caddy:2019/id/{}", route_id))
            .send()
            .await;

        // Also remove file-based config
        let config_path = self.config_dir.join(format!(
            "{}.caddy",
            deployment_url.replace('.', "_").replace('*', "_wildcard")
        ));

        if config_path.exists() {
            tokio::fs::remove_file(&config_path).await?;
            tracing::info!("Removed Caddy config for {}", deployment_url);
        }

        Ok(())
    }

    /// Generate a Caddy config for a custom domain pointing to a project
    pub async fn add_custom_domain(
        &self,
        custom_domain: &str,
        project_name: &str,
        _artifacts_dir: &str,
    ) -> anyhow::Result<()> {
        let deployment_url = format!("{}.{}", project_name, self.base_domain);
        
        let config = format!(
            r#"# Custom domain: {custom_domain} → {project_name}
{custom_domain} {{
    # Redirect to the project's deployment
    reverse_proxy {deployment_url} {{
        header_up Host {{upstream_host}}
    }}
    
    # TLS
    tls {{
        protocols tls1.2 tls1.3
    }}
    
    # Security headers
    header X-Content-Type-Options "nosniff"
    header X-Frame-Options "DENY"
}}
"#,
            custom_domain = custom_domain,
            project_name = project_name,
            deployment_url = deployment_url,
        );

        let config_path = self.config_dir.join(format!(
            "custom_{}.caddy",
            custom_domain.replace('.', "_")
        ));
        tokio::fs::write(&config_path, config).await?;

        tracing::info!("Generated custom domain config: {} → {}", custom_domain, project_name);
        Ok(())
    }

    /// Reload Caddy config via admin API (POST /load with adapted Caddyfile).
    /// Reads the Caddyfile from the mounted volume, adapts it via Caddy's
    /// /adapt endpoint, then loads the JSON config. Faster than container restart.
    pub async fn reload(&self) -> anyhow::Result<()> {
        // No-op if routes were added via admin API (atomic updates, no restart needed)
        if self.api_mode.load(Ordering::Relaxed) {
            tracing::debug!("Caddy admin API mode — no reload needed");
            return Ok(());
        }

        let client = reqwest::Client::new();

        // Read the Caddyfile from the shared volume mount
        let caddyfile = match tokio::fs::read_to_string("/app/caddy/Caddyfile").await {
            Ok(content) => content,
            Err(_) => tokio::fs::read_to_string("./caddy/Caddyfile").await.unwrap_or_default(),
        };

        // Adapt Caddyfile to JSON via Caddy admin API
        let adapt_resp = client
            .post("http://caddy:2019/adapt")
            .header("Content-Type", "text/caddyfile")
            .body(caddyfile)
            .send()
            .await;

        match adapt_resp {
            Ok(resp) if resp.status().is_success() => {
                let config_json = resp.text().await.unwrap_or_default();
                // Load the adapted JSON config
                let load_resp = client
                    .post("http://caddy:2019/load")
                    .header("Content-Type", "application/json")
                    .body(config_json)
                    .send()
                    .await;
                match load_resp {
                    Ok(r) if r.status().is_success() => {
                        tracing::info!("Caddy config reloaded via admin API");
                    }
                    Ok(r) => {
                        tracing::warn!("Caddy load returned {}{}, falling back to restart", r.status(), r.text().await.unwrap_or_default());
                        self.reload_via_restart().await?;
                    }
                    Err(e) => {
                        tracing::warn!("Caddy load failed: {}, falling back to restart", e);
                        self.reload_via_restart().await?;
                    }
                }
            }
            _ => {
                // Adapt failed — likely new .caddy files aren't in the Caddyfile yet
                // Fall back to container restart which re-evaluates the import glob
                tracing::debug!("Caddy adapt failed, falling back to restart");
                self.reload_via_restart().await?;
            }
        }

        Ok(())
    }

    /// Fallback: restart Caddy container to pick up new import files
    async fn reload_via_restart(&self) -> anyhow::Result<()> {
        let docker = bollard::Docker::connect_with_local_defaults()?;
        use bollard::container::RestartContainerOptions;
        docker
            .restart_container("vercel-clone-caddy-1", Some(RestartContainerOptions { t: 2 }))
            .await?;
        tracing::info!("Caddy container restarted (fallback)");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        Ok(())
    }
}
