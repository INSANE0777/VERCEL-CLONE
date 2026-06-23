mod api;
mod builder;
mod config;
mod db;
mod edge;
mod models;
mod queue;
mod storage;

use builder::warm_pool::WarmPool;

use config::AppConfig;
use db::Database;
use edge::EdgeRouter;
use queue::BuildQueue;
use storage::ArtifactStore;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::SystemTime;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: AppConfig,
    pub queue: BuildQueue,
    pub artifacts: ArtifactStore,
    pub router: EdgeRouter,
    pub warm_pool: Option<Arc<WarmPool>>,
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
    let db = Database::new(&config.database_url).await?;
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

    let state = Arc::new(AppState {
        db: db.clone(),
        config: config.clone(),
        queue: queue.clone(),
        artifacts: artifacts.clone(),
        router: router.clone(),
        warm_pool: warm_pool.clone(),
        active_builds: Arc::new(AtomicUsize::new(0)),
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

        tokio::spawn(async move {
            tracing::info!("Build worker {} starting", i);
            builder::run_build_worker(worker_db, worker_queue, worker_artifacts, worker_router, worker_config, worker_pool).await;
        });
    }

    // Start API server
    let app = api::create_router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!("API server listening on 0.0.0.0:{}", config.port);
    axum::serve(listener, app).await?;

    Ok(())
}
