use anyhow::Context;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,

    // ── PostgreSQL ──
    pub database_url: String,

    // ── NATS JetStream ──
    pub nats_url: String,
    pub nats_stream_name: String,
    pub nats_consumer_name: String,

    // ── S3 / Object Storage ──
    pub s3_endpoint: String,
    pub s3_bucket: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_region: String,

    // ── GitHub ──
    pub github_app_id: String,
    pub github_app_private_key: String,
    pub github_webhook_secret: String,
    pub github_token: String,

    // ── Platform ──
    pub base_domain: String,
    pub docker_image: String,
    pub build_memory_limit: i64,
    pub build_timeout_secs: u64,

    // ── Edge / Serving ──
    pub caddy_config_dir: String,
    pub artifacts_dir: String, // local fallback if S3 unavailable

    // ── Scaling ──
    pub max_concurrent_builds: usize,
    pub warm_pool_size: usize,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            port: env_or("PORT", "3000").parse().context("PORT must be a number")?,

            // PostgreSQL
            database_url: env_or(
                "DATABASE_URL",
                "postgres://postgres:postgres@localhost:5432/vercel_clone",
            ),

            // NATS
            nats_url: env_or("NATS_URL", "nats://localhost:4222"),
            nats_stream_name: env_or("NATS_STREAM_NAME", "BUILDS"),
            nats_consumer_name: env_or("NATS_CONSUMER_NAME", "build-workers"),

            // S3
            s3_endpoint: env_or("S3_ENDPOINT", "http://localhost:9000"),
            s3_bucket: env_or("S3_BUCKET", "vercel-artifacts"),
            s3_access_key: env_or("S3_ACCESS_KEY", "minioadmin"),
            s3_secret_key: env_or("S3_SECRET_KEY", "minioadmin"),
            s3_region: env_or("S3_REGION", "us-east-1"),

            // GitHub
            github_app_id: env_or("GITHUB_APP_ID", ""),
            github_app_private_key: env_or("GITHUB_APP_PRIVATE_KEY", ""),
            github_webhook_secret: env_or("GITHUB_WEBHOOK_SECRET", "dev-secret"),
            github_token: env_or("GITHUB_TOKEN", ""),

            // Platform
            base_domain: env_or("BASE_DOMAIN", "localhost"),
            docker_image: env_or("DOCKER_BUILD_IMAGE", "node:20-slim"),
            build_memory_limit: env_or("BUILD_MEMORY_LIMIT", "2147483648")
                .parse()
                .context("BUILD_MEMORY_LIMIT must be a number")?,
            build_timeout_secs: env_or("BUILD_TIMEOUT_SECS", "600")
                .parse()
                .context("BUILD_TIMEOUT_SECS must be a number")?,

            // Edge
            caddy_config_dir: env_or("CADDY_CONFIG_DIR", "./caddy/configs"),
            artifacts_dir: env_or("ARTIFACTS_DIR", "./artifacts"),

            // Scaling
            max_concurrent_builds: env_or("MAX_CONCURRENT_BUILDS", "4")
                .parse()
                .context("MAX_CONCURRENT_BUILDS must be a number")?,
            warm_pool_size: env_or("WARM_POOL_SIZE", "2")
                .parse()
                .context("WARM_POOL_SIZE must be a number")?,
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
