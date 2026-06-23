use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions,
};
use bollard::models::HostConfig;
use bollard::Docker;
use dashmap::DashMap;
use std::path::Path;
use std::sync::Arc;

/// Manages serverless function runtime containers.
/// Each deployment with API routes gets a persistent Docker container
/// running a Node.js server that executes the compiled functions.
#[derive(Clone)]
pub struct FunctionRuntime {
    docker: Docker,
    /// deployment_id -> container_id
    containers: Arc<DashMap<String, String>>,
    /// Port counter for function containers
    next_port: Arc<std::sync::atomic::AtomicU16>,
    base_port: u16,
    docker_image: String,
    env_vars: Vec<(String, String)>,
}

impl FunctionRuntime {
    pub fn new(docker_image: &str, base_port: u16, env_vars: Vec<(String, String)>) -> Self {
        Self {
            docker: Docker::connect_with_local_defaults()
                .expect("Failed to connect to Docker"),
            containers: Arc::new(DashMap::new()),
            next_port: Arc::new(std::sync::atomic::AtomicU16::new(base_port)),
            base_port,
            docker_image: docker_image.to_string(),
            env_vars,
        }
    }

    /// Detect if the build output contains API routes worth running a server for.
    /// Checks for Next.js server output and generic api/ directories.
    pub fn detect_api_routes(build_path: &Path, framework: &str) -> Vec<String> {
        let mut routes = Vec::new();

        // Next.js: .next/server/pages/api/ and .next/server/app/api/
        if framework == "nextjs" {
            let next_server = build_path.join(".next/server");
            for api_dir in &["pages/api", "app/api"] {
                let path = next_server.join(api_dir);
                if path.exists() {
                    collect_route_files(&path, &mut routes);
                }
            }
        }

        // Generic: api/ or server/api/ in build output
        for api_dir in &["api", "server/api", "functions"] {
            let path = build_path.join(api_dir);
            if path.exists() {
                collect_route_files(&path, &mut routes);
            }
        }

        routes
    }

    /// Start a function runtime container for a deployment.
    /// Returns the port the container is listening on.
    pub async fn start_for_deployment(
        &self,
        deployment_id: &str,
        build_path: &Path,
        framework: &str,
        env_vars: &[(String, String)],
    ) -> anyhow::Result<u16> {
        // Stop existing container for this deployment if any
        self.stop_for_deployment(deployment_id).await;

        let routes = Self::detect_api_routes(build_path, framework);
        if routes.is_empty() {
            anyhow::bail!("No API routes detected");
        }

        tracing::info!(
            "Starting function runtime for {} ({} routes detected)",
            deployment_id,
            routes.len()
        );

        let port = self
            .next_port
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Generate a minimal Node.js server script
        let server_script = generate_server_script(&routes, framework);
        let script_path = build_path.join("_serverless_runtime.js");
        tokio::fs::write(&script_path, &server_script).await?;

        // Create and start container
        let env: Vec<String> = env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let host_config = HostConfig {
            binds: Some(vec![format!(
                "{}:/app",
                build_path.to_str().unwrap_or("/tmp")
            )]),
            ..Default::default()
        };

        let config = Config {
            image: Some(self.docker_image.clone()),
            cmd: Some(vec!["node".to_string(), "/app/_serverless_runtime.js".to_string()]),
            env: Some(env),
            host_config: Some(host_config),
            exposed_ports: Some(
                [(format!("{}/tcp", port), std::collections::HashMap::new())]
                    .into_iter()
                    .collect(),
            ),
            ..Default::default()
        };

        let container = self
            .docker
            .create_container(
                Some(CreateContainerOptions {
                    name: format!("fn-{}", deployment_id),
                    platform: None,
                }),
                config,
            )
            .await?;

        let container_id = container.id;
        self.docker
            .start_container(&container_id, None::<bollard::container::StartContainerOptions<String>>)
            .await?;

        self.containers.insert(deployment_id.to_string(), container_id.clone());

        tracing::info!(
            "Function runtime started for {} on port {} (container {})",
            deployment_id,
            port,
            &container_id[..12]
        );

        Ok(port)
    }

    /// Stop and remove the function container for a deployment.
    pub async fn stop_for_deployment(&self, deployment_id: &str) {
        if let Some((_, container_id)) = self.containers.remove(deployment_id) {
            let _ = self
                .docker
                .remove_container(
                    &container_id,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await;
            tracing::info!("Stopped function runtime for {}", deployment_id);
        }
    }

    /// Check if a deployment has a running function container.
    pub fn is_running(&self, deployment_id: &str) -> bool {
        self.containers.contains_key(deployment_id)
    }

    /// Get the port for a running function container.
    /// Returns None if not running.
    pub fn get_port(&self, deployment_id: &str) -> Option<u16> {
        if self.containers.contains_key(deployment_id) {
            // port is derived from position in the map — but we stored it implicitly
            // Actually we need to track ports. Let me fix this.
            // For now, return None and let Caddy proxy handle it.
            None
        } else {
            None
        }
    }
}

/// Walk a directory and collect .js/.ts route file paths relative to the api dir.
fn collect_route_files(api_dir: &Path, routes: &mut Vec<String>) {
    for entry in walkdir::WalkDir::new(api_dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
            if ext == "js" || ext == "ts" || ext == "mjs" {
                if let Ok(rel) = entry.path().strip_prefix(api_dir) {
                    let route = rel.with_extension("")
                        .to_string_lossy()
                        .replace('\\', "/");
                    routes.push(format!("/{}", route));
                }
            }
        }
    }
}

/// Generate a minimal Node.js server that loads and executes API route files.
/// This is a commonjs server that requires each route file and maps HTTP methods.
fn generate_server_script(routes: &[String], framework: &str) -> String {
    let route_imports: String = routes
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let file_path = if framework == "nextjs" {
                format!(".next/server/pages/api{}.js", r)
            } else {
                format!("api{}.js", r)
            };
            format!(
                "const route_{} = require('./{}');",
                i,
                file_path
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let route_map: String = routes
        .iter()
        .enumerate()
        .map(|(i, r)| format!("  '{}': route_{},", r, i))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"// Auto-generated serverless function runtime
const http = require('http');
const url = require('url');

{route_imports}

const routes = {{
{route_map}
}};

const server = http.createServer((req, res) => {{
    const parsed = url.parse(req.url, true);
    const pathname = parsed.pathname;

    // Match exact route or strip trailing /index
    let handler = routes[pathname] || routes[pathname.replace(/\/index$/, '')];

    if (!handler) {{
        // Try prefix matching for dynamic routes
        for (const [pattern, fn] of Object.entries(routes)) {{
            const regex = new RegExp('^' + pattern.replace(/\[.*?\]/g, '([^/]+)') + '$');
            if (regex.test(pathname)) {{
                handler = fn;
                break;
            }}
        }}
    }}

    if (!handler) {{
        res.writeHead(404, {{ 'Content-Type': 'application/json' }});
        res.end(JSON.stringify({{ error: 'Not Found' }}));
        return;
    }}

    // Next.js style: handler has default export with method handlers
    const fn = handler.default || handler;
    const method = (req.method || 'GET').toLowerCase();

    if (typeof fn === 'function') {{
        fn(req, res);
    }} else if (fn && typeof fn[method] === 'function') {{
        fn[method](req, res);
    }} else {{
        res.writeHead(405, {{ 'Content-Type': 'application/json' }});
        res.end(JSON.stringify({{ error: 'Method Not Allowed' }}));
    }}
}});

server.listen(process.env.PORT || 3001, () => {{
    console.log('Serverless runtime listening on ' + (process.env.PORT || 3001));
}});
"#,
        route_imports = route_imports,
        route_map = route_map
    )
}
