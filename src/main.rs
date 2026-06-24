mod api;
mod builder;
mod config;
mod dashboard;
mod db;
mod edge;
mod github;
mod models;
mod queue;
mod storage;
#[cfg(test)]
mod tests;

use builder::warm_pool::WarmPool;

use config::AppConfig;
use db::Database;
use edge::EdgeRouter;
use github::PrBot;
use queue::BuildQueue;
use storage::ArtifactStore;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tracing_subscriber::EnvFilter;

/// Remove orphaned build-* containers from previous runs.
async fn cleanup_orphaned_containers() {
    let docker = match bollard::Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(_) => return,
    };

    let containers = match docker.list_containers(Some(bollard::container::ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    })).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to list containers for cleanup: {}", e);
            return;
        }
    };

    let mut cleaned = 0;
    for container in &containers {
        if let Some(names) = &container.names {
            for name in names {
                if name.starts_with("/build-") || name.starts_with("build-") {
                    if let Some(id) = &container.id {
                        let _ = docker.remove_container(id, Some(bollard::container::RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        })).await;
                        cleaned += 1;
                    }
                }
            }
        }
    }

    if cleaned > 0 {
        tracing::info!("Cleaned up {} orphaned build containers", cleaned);
    } else {
        tracing::info!("No orphaned build containers found");
    }
}

/// Wait for SIGTERM/SIGINT, then wait for in-flight builds to finish (30s timeout).
async fn shutdown_signal(active_builds: Arc<AtomicUsize>) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => tracing::info!("Received SIGTERM"),
            _ = sigint.recv() => tracing::info!("Received SIGINT"),
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        tracing::info!("Received Ctrl+C");
    }

    tracing::info!("Shutting down, waiting for in-flight builds...");
    let timeout = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();
    loop {
        let count = active_builds.load(Ordering::Relaxed);
        if count == 0 {
            tracing::info!("All builds completed, exiting");
            break;
        }
        if start.elapsed() >= timeout {
            tracing::warn!("Shutdown timeout reached, forcing exit ({} builds still active)", count);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: AppConfig,
    pub queue: BuildQueue,
    pub artifacts: ArtifactStore,
    pub router: EdgeRouter,
    pub warm_pool: Option<Arc<WarmPool>>,
    pub pr_bot: Option<Arc<PrBot>>,
    pub active_builds: Arc<AtomicUsize>,
    pub start_time: SystemTime,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("vercel_clone=debug".parse()?))
        .init();

    let config = AppConfig::from_env()?;
    tracing::info!("vercel-clone v{} starting on port {}", env!("CARGO_PKG_VERSION"), config.port);

    // PostgreSQL database
    let db = Database::new(&config.database_url, &config.encryption_key).await?;
    tracing::info!("PostgreSQL connected");

    // NATS JetStream queue
    let queue = BuildQueue::connect(&config.nats_url, &config.nats_stream_name, &config.nats_consumer_name).await?;
    tracing::info!("NATS JetStream connected");

    // S3 / MinIO artifact storage
    let artifacts = ArtifactStore::new(
        &config.s3_endpoint,
        &config.s3_bucket,
        &config.s3_access_key,
        &config.s3_secret_key,
        &config.s3_region,
        &config.artifacts_dir,
    );

    // Caddy edge router
    let router = EdgeRouter::new(&config.caddy_config_dir, &config.base_domain);
    tracing::info!("Edge router initialized");

    // Firecracker warm pool (Linux + KVM only)
    let warm_pool: Option<Arc<WarmPool>> = if builder::firecracker::FirecrackerRunner::is_available().await {
        let fc_dir = PathBuf::from("/tmp/vercel-clone-firecracker");
        let runner = builder::firecracker::FirecrackerRunner::new(&fc_dir);
        match runner.prepare(&config.docker_image).await {
            Ok(_) => {
                let pool = WarmPool::new(runner, config.warm_pool_size);
                pool.clone().start();
                tracing::info!("Firecracker warm pool started (size: {})", config.warm_pool_size);
                Some(pool)
            }
            Err(e) => {
                tracing::warn!("Firecracker preparation failed ({}), warm pool disabled", e);
                None
            }
        }
    } else {
        tracing::info!("Firecracker unavailable — warm pool disabled (Docker fallback)");
        None
    };

    // GitHub PR bot (optional — requires GITHUB_TOKEN)
    let pr_bot = PrBot::new(&config.github_token).map(Arc::new);
    if pr_bot.is_some() {
        tracing::info!("GitHub PR bot enabled");
    } else {
        tracing::info!("GitHub PR bot disabled (no GITHUB_TOKEN)");
    }

    // Cleanup orphaned build containers from previous runs
    cleanup_orphaned_containers().await;

    let active_builds = Arc::new(AtomicUsize::new(0));

    let state = Arc::new(AppState {
        db: db.clone(),
        config: config.clone(),
        queue: queue.clone(),
        artifacts: artifacts.clone(),
        router: router.clone(),
        warm_pool: warm_pool.clone(),
        pr_bot: pr_bot.clone(),
        active_builds: active_builds.clone(),
        start_time: SystemTime::now(),
    });

    // Start build workers
    for i in 0..config.max_concurrent_builds {
        let worker_db = db.clone();
        let worker_queue = queue.clone();
        let worker_artifacts = artifacts.clone();
        let worker_router = router.clone();
        let worker_config = config.clone();
        let worker_pool = warm_pool.clone();
        let worker_builds = active_builds.clone();

        tokio::spawn(async move {
            tracing::info!("Build worker {} starting", i);
            builder::run_build_worker(worker_db, worker_queue, worker_artifacts, worker_router, worker_config, worker_pool, worker_builds).await;
        });
    }

    // Start API server with graceful shutdown
    let app = api::create_router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!("API server listening on 0.0.0.0:{}", config.port);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(active_builds))
        .await?;

    Ok(())
}
