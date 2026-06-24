use std::path::PathBuf;

pub mod middleware;

use middleware::MiddlewareRule;

/// Manages Caddy configuration for serving deployed sites.
/// Caddy provides automatic HTTPS via Let's Encrypt.
#[derive(Clone)]
pub struct EdgeRouter {
    config_dir: PathBuf,
    base_domain: String,
}

impl EdgeRouter {
    pub fn new(config_dir: &str, base_domain: &str) -> Self {
        let config_dir = PathBuf::from(config_dir);
        std::fs::create_dir_all(&config_dir).expect("Failed to create Caddy config directory");
        Self {
            config_dir,
            base_domain: base_domain.to_string(),
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
{deployment_url} {{
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
{deployment_url} {{
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

        tracing::info!(
            "Generated Caddy config: {} → {}",
            deployment_url,
            config_path.display()
        );
        Ok(())
    }

    /// Remove a deployment's Caddy config
    pub async fn remove_deployment(&self, deployment_url: &str) -> anyhow::Result<()> {
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

    /// Signal Caddy to reload configuration (re-parses Caddyfile + imports)
    pub async fn reload(&self) -> anyhow::Result<()> {
        // The simplest reliable way to reload Caddy with new import files
        // is to restart the container. Caddy's admin API adapt endpoint
        // requires the Caddyfile content, but the API container doesn't have it.
        // A container restart re-parses everything including new .caddy imports.
        let docker = bollard::Docker::connect_with_local_defaults()?;
        
        // Restart the caddy container
        use bollard::container::RestartContainerOptions;
        docker.restart_container("vercel-clone-caddy-1", Some(RestartContainerOptions { t: 2 })).await?;
        tracing::info!("Caddy container restarted to pick up new deployment configs");
        
        // Give Caddy a moment to come back up
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        Ok(())
    }
}
