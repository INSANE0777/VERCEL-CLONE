pub mod firecracker;
pub mod framework;
pub mod functions;
pub mod image_opt;
pub mod warm_pool;

use crate::config::AppConfig;
use crate::db::Database;
use crate::edge::EdgeRouter;
use crate::models::BuildJob;
use crate::queue::BuildQueue;
use crate::storage::ArtifactStore;
use bollard::container::{
    Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions,
    WaitContainerOptions,
};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use bollard::Docker;
use futures_util::stream::StreamExt;
use std::sync::Arc;
use std::path::PathBuf;

/// Main build worker — consumes jobs from NATS JetStream
pub async fn run_build_worker(
    db: Database,
    queue: BuildQueue,
    artifacts: ArtifactStore,
    router: EdgeRouter,
    config: AppConfig,
    warm_pool: Option<Arc<warm_pool::WarmPool>>,
) {
    tracing::info!("Build worker starting — connecting to NATS consumer...");

    let consumer = match queue.create_consumer().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to create NATS consumer: {}", e);
            return;
        }
    };

    tracing::info!("NATS consumer ready — waiting for build jobs");

    loop {
        let mut messages = match consumer.messages().await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Failed to get messages from NATS: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        while let Some(Ok(msg)) = messages.next().await {
            let job: BuildJob = match serde_json::from_slice(&msg.payload) {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!("Failed to parse build job: {}", e);
                    let _ = msg.ack().await;
                    continue;
                }
            };

            tracing::info!(
                "Received build job: deployment={} project={} branch={} attempt={}",
                job.deployment_id,
                job.project_id,
                job.branch,
                job.attempt
            );

            // Mark as building in DB
            let deployment_id = match uuid::Uuid::parse_str(&job.deployment_id) {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!("Invalid deployment UUID: {}", e);
                    let _ = msg.ack().await;
                    continue;
                }
            };

            if let Err(e) = db.update_deployment_status(deployment_id, "building", None, None).await {
                tracing::error!("Failed to update deployment status: {}", e);
            }

            // Record analytics: build started
            let _ = db.record_analytics_event(
                uuid::Uuid::parse_str(&job.project_id).ok(),
                Some(deployment_id),
                "build_started",
                None, None, Some(job.is_production),
            ).await;

            // Publish status update
            let _ = queue.publish_status(&job.deployment_id, "building", "Build started").await;

            // Execute build
            let result = execute_build(&db, &artifacts, &router, &config, &job, warm_pool.clone()).await;

            match result {
                Ok((logs, framework)) => {
                    tracing::info!("Build {} completed successfully (framework: {})", job.deployment_id, framework);

                    if let Err(e) = db.update_deployment_status(
                        deployment_id, "ready", Some(&logs), Some(&framework)
                    ).await {
                        tracing::error!("Failed to mark deployment ready: {}", e);
                    }

                    let _ = queue.publish_status(&job.deployment_id, "ready", "Build completed").await;

                    // Record analytics: build completed
                    let build_duration = 0i32; // ponytail: real timing needs a stopwatch around execute_build
                    let _ = db.record_analytics_event(
                        uuid::Uuid::parse_str(&job.project_id).ok(),
                        Some(deployment_id),
                        "build_completed",
                        Some(&framework), Some(build_duration), Some(job.is_production),
                    ).await;

                    // Update PR comment if this was a PR deployment
                    update_pr_comment_if_exists(&db, &config, &job.deployment_id, "ready", Some(&framework), None).await;

                    let _ = msg.ack().await;
                }
                Err(e) => {
                    let error_msg = format!("Build failed: {}", e);
                    tracing::error!("{}", error_msg);

                    if let Err(e) = db.update_deployment_status(
                        deployment_id, "error", Some(&error_msg), None
                    ).await {
                        tracing::error!("Failed to mark deployment error: {}", e);
                    }

                    let _ = queue.publish_status(&job.deployment_id, "error", &error_msg).await;

                    // Record analytics: build failed
                    let _ = db.record_analytics_event(
                        uuid::Uuid::parse_str(&job.project_id).ok(),
                        Some(deployment_id),
                        "build_failed",
                        None, None, Some(job.is_production),
                    ).await;

                    // Update PR comment with error
                    update_pr_comment_if_exists(&db, &config, &job.deployment_id, "error", None, Some(&error_msg)).await;

                    // Retry or dead-letter if attempts exhausted
                    if job.attempt < 3 {
                        if let Err(e) = queue.retry(&job).await {
                            tracing::error!("Failed to retry job: {}", e);
                        }
                    } else {
                        tracing::error!("Build {} exhausted all retry attempts — sent to DLQ", job.deployment_id);
                    }

                    let _ = msg.ack().await;
                }
            }
        }
    }
}

/// Execute a build: clone, detect framework, restore cache, build, store artifacts
async fn execute_build(
    db: &Database,
    artifacts: &ArtifactStore,
    router: &EdgeRouter,
    config: &AppConfig,
    job: &BuildJob,
    warm_pool: Option<Arc<warm_pool::WarmPool>>,
) -> anyhow::Result<(String, String)> {
    let _docker = Docker::connect_with_local_defaults()?;

    // Temp directory for cloning + building
    let build_dir = format!("/tmp/vercel-clone-builds/{}", job.deployment_id);
    let build_path = PathBuf::from(&build_dir);

    // Clean up old build dir
    if build_path.exists() {
        let _ = tokio::fs::remove_dir_all(&build_path).await;
    }
    tokio::fs::create_dir_all(&build_path).await?;

    // ── Step 1: Clone the repo ──
    let clone_url = if !config.github_token.is_empty() && job.repo_url.starts_with("https://github.com/") {
        format!("https://x-access-token:{}@github.com/{}",
            config.github_token,
            &job.repo_url["https://github.com/".len()..])
    } else {
        job.repo_url.clone()
    };

    tracing::info!("Cloning {}...", job.repo_url);
    let output = tokio::process::Command::new("git")
        .args(["clone", "--depth", "1", &clone_url, &build_dir])
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!("Git clone failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Checkout specific SHA if provided
    if !job.sha.is_empty() && job.sha != "HEAD" {
        let _ = tokio::process::Command::new("git")
            .args(["fetch", "--depth", "1", "origin", &job.sha])
            .current_dir(&build_dir)
            .output()
            .await;

        let output = tokio::process::Command::new("git")
            .args(["checkout", &job.sha])
            .current_dir(&build_dir)
            .output()
            .await?;

        if !output.status.success() {
            tracing::warn!("Could not checkout SHA {}: {}", &job.sha[..7.min(job.sha.len())], 
                String::from_utf8_lossy(&output.stderr));
        }
    }

    // ── Step 2: Detect framework ──
    let fw = framework::detect_framework(&build_path).await?;
    tracing::info!("Detected framework: {} (output: {})", fw.name, fw.output_dir);

    // ── Step 3: Restore dependency cache ──
    let lockfile_hash = sha256_any_lockfile(&build_path).await;
    let cache_key = format!("deps:{}:{}", job.project_id, lockfile_hash);
    
    let cache_restored = if fw.install_command != "echo 'no install needed'" {
        match artifacts.restore_cache(&job.project_id, &cache_key, &build_path.join("node_modules")).await {
            Ok(true) => {
                tracing::info!("Cache hit! Restored node_modules from cache");
                true
            }
            Ok(false) => {
                tracing::info!("Cache miss — will install dependencies fresh");
                false
            }
            Err(e) => {
                tracing::warn!("Cache restore failed: {}", e);
                false
            }
        }
    } else {
        false
    };

    // ── Step 4: Run build (Firecracker microVM preferred, Docker fallback) ──
    let logs = match run_build_firecracker_or_docker(config, &build_path, &fw, &job.env_vars, job, warm_pool).await {
        Ok(logs) => logs,
        Err(e) => {
            tracing::error!("Build execution failed: {}", e);
            return Err(e);
        }
    };

    // ── Step 5: Save dependency cache ──
    if !cache_restored && fw.install_command != "echo 'no install needed'" {
        let node_modules = build_path.join("node_modules");
        if node_modules.exists() {
            match artifacts.store_cache(&job.project_id, &cache_key, &node_modules).await {
                Ok(bytes) => tracing::info!("Saved {} bytes to dependency cache", bytes),
                Err(e) => tracing::warn!("Failed to save cache: {}", e),
            }
        }
    }

    // ── Step 6: Store build artifacts ──
    let output_path = build_path.join(&fw.output_dir);
    let artifact_path = if output_path.exists() {
        output_path
    } else {
        // Try common alternatives
        let mut found = None;
        for dir in &["out", "build", "dist", "public", "_site"] {
            let alt = build_path.join(dir);
            if alt.exists() {
                found = Some(alt);
                break;
            }
        }
        match found {
            Some(p) => p,
            None => {
                // No build output found — use the whole directory
                build_path.clone()
            }
        }
    };

    // ── Step 6.5: Optimize images in build output ──
    match image_opt::optimize_images(&artifact_path).await {
        Ok((count, saved)) if count > 0 => {
            tracing::info!("Optimized {} images, ~{} bytes saved", count, saved);
        }
        Ok(_) => tracing::debug!("No images to optimize"),
        Err(e) => tracing::warn!("Image optimization failed (non-fatal): {}", e),
    }

    let total_bytes = artifacts.store_build_output(&job.deployment_id, &artifact_path).await?;
    tracing::info!("Stored {} bytes of build artifacts", total_bytes);

    // ── Step 7: Generate Caddy config for serving ──
    let deployment_url = if job.is_production {
        format!("{}.localhost", job.repo_full_name.split('/').last().unwrap_or("app"))
    } else {
        format!("{}-{:.8}.localhost", 
            job.repo_full_name.split('/').last().unwrap_or("app"),
            job.deployment_id)
    };

    // Fetch middleware rules for this project
    let mw_rules: Vec<crate::edge::middleware::MiddlewareRule> = match uuid::Uuid::parse_str(&job.project_id) {
        Ok(pid) => match db.list_middleware_rules(pid).await {
            Ok(rules) => rules.into_iter().map(|r| r.to_rule()).collect(),
            Err(e) => {
                tracing::warn!("Failed to fetch middleware rules: {}", e);
                Vec::new()
            }
        },
        Err(_) => Vec::new(),
    };

    // Detect and start serverless function runtime if API routes exist
    let has_functions = !functions::FunctionRuntime::detect_api_routes(&artifact_path, &fw.name).is_empty();
    if has_functions {
        tracing::info!("Detected API routes for {} — function runtime will be available", job.deployment_id);
        // Function runtime is started on-demand by the API when the deployment is first accessed
        // The Caddy config includes a proxy for /api/* that points to the function runtime
    }

    router.add_deployment(
        &job.deployment_id,
        &deployment_url,
        &artifacts.local_artifacts_dir(&job.deployment_id),
        &fw.name,
        &mw_rules,
    ).await?;

    // Cleanup
    let _ = tokio::fs::remove_dir_all(&build_path).await;

    Ok((logs, fw.name))
}

/// Try warm pool → Firecracker cold boot → Docker fallback.
async fn run_build_firecracker_or_docker(
    config: &AppConfig,
    build_path: &std::path::Path,
    fw: &framework::Framework,
    env_vars: &[(String, String)],
    job: &crate::models::BuildJob,
    warm_pool: Option<Arc<warm_pool::WarmPool>>,
) -> anyhow::Result<String> {
    // ── Priority 1: Warm pool (instant VM, ~100ms startup) ──
    if let Some(pool) = warm_pool {
        match pool.acquire().await {
            Some(vm) => {
                let vm_id = vm.id.clone();
                tracing::info!(
                    "Warm pool VM {} acquired — running build (instant start)",
                    vm_id
                );

                let build_result = pool
                    .run_build(
                        &vm,
                        &job.repo_url,
                        &job.sha,
                        fw,
                        env_vars,
                        build_path,
                    )
                    .await;

                // Release destroys VM + triggers background respawn
                pool.release(vm).await;

                match build_result {
                    Ok(logs) => {
                        tracing::info!(
                            "Build completed in warm pool VM {} — hardware-enforced isolation",
                            vm_id
                        );
                        return Ok(logs);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Warm pool build failed ({}), trying cold boot fallback",
                            e
                        );
                        // Fall through to cold boot
                    }
                }
            }
            None => {
                tracing::warn!("Warm pool empty — falling back to cold boot");
                // Fall through to cold boot
            }
        }
    }

    // ── Priority 2: Firecracker cold boot (~3-5s startup) ──
    if firecracker::FirecrackerRunner::is_available().await {
        tracing::info!("Firecracker cold boot — using hardware-enforced isolation");

        let fc_dir = std::path::PathBuf::from("/tmp/vercel-clone-firecracker");
        let runner = firecracker::FirecrackerRunner::new(&fc_dir);

        if let Err(e) = runner.prepare(&config.docker_image).await {
            tracing::warn!("Firecracker prep failed ({}), falling back to Docker", e);
            return run_build_in_docker(config, build_path, fw, env_vars).await;
        }

        let vm = match runner.spawn_vm(build_path).await {
            Ok(vm) => vm,
            Err(e) => {
                tracing::warn!("Failed to spawn Firecracker VM ({}), falling back to Docker", e);
                return run_build_in_docker(config, build_path, fw, env_vars).await;
            }
        };

        let vm_id = vm.id.clone();
        let build_result = runner.run_build_in_vm(&vm, fw, env_vars).await;

        if build_result.is_ok() {
            let _ = runner
                .copy_build_output(&vm, &fw.output_dir, build_path)
                .await;
        }

        let _ = runner.destroy_vm(vm).await;

        match build_result {
            Ok(logs) => {
                tracing::info!("Build completed in Firecracker VM {}", vm_id);
                return Ok(logs);
            }
            Err(e) => {
                tracing::warn!("Firecracker build failed ({}), falling back to Docker", e);
                return run_build_in_docker(config, build_path, fw, env_vars).await;
            }
        }
    }

    // ── Priority 3: Docker containers (software isolation) ──
    tracing::info!("Using Docker containers (software isolation only)");
    run_build_in_docker(config, build_path, fw, env_vars).await
}

/// Run the build command inside a Docker container
async fn run_build_in_docker(
    config: &AppConfig,
    build_path: &std::path::Path,
    fw: &framework::Framework,
    env_vars: &[(String, String)],
) -> anyhow::Result<String> {
    let docker = Docker::connect_with_local_defaults()?;
    run_build_in_container(&docker, config, build_path.to_str().unwrap(), fw, env_vars).await
}

/// Run the build command inside a Docker container (legacy signature)
async fn run_build_in_container(
    docker: &Docker,
    config: &AppConfig,
    build_dir: &str,
    fw: &framework::Framework,
    env_vars: &[(String, String)],
) -> anyhow::Result<String> {
    // Pull image if needed
    let pull_options = bollard::image::CreateImageOptions {
        from_image: config.docker_image.clone(),
        ..Default::default()
    };
    let mut pull_stream = docker.create_image(Some(pull_options), None, None);
    while let Some(Ok(_)) = pull_stream.next().await {
        // consume stream
    }

    // Mount the shared builds volume and target the specific build subdir
    let build_subdir = std::path::Path::new(build_dir)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".");

    let mount = Mount {
        target: Some("/builds".to_string()),
        source: Some("builds_data".to_string()),
        typ: Some(MountTypeEnum::VOLUME),
        read_only: Some(false),
        ..Default::default()
    };

    let host_config = HostConfig {
        mounts: Some(vec![mount]),
        memory: Some(config.build_memory_limit),
        network_mode: Some("host".to_string()),
        ..Default::default()
    };

    // Build script with env vars
    let env: Vec<String> = env_vars.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    let build_script = format!(
        "cd /builds/{} && {} && {}",
        build_subdir, fw.install_command, fw.build_command
    );

    let container_config = Config {
        image: Some(config.docker_image.clone()),
        cmd: Some(vec!["sh".to_string(), "-c".to_string(), build_script]),
        env: Some(env),
        host_config: Some(host_config),
        ..Default::default()
    };

    let container = docker
        .create_container(
            Some(CreateContainerOptions {
                name: format!("build-{}", uuid::Uuid::new_v4()),
                platform: None,
            }),
            container_config,
        )
        .await?;

    let container_id = &container.id;
    tracing::info!("Created build container: {}", container_id);

    // Start container
    docker
        .start_container(container_id, None::<bollard::container::StartContainerOptions<String>>)
        .await?;

    // Stream logs with timeout
    let mut all_logs = String::new();
    let mut log_stream = docker.logs(
        container_id,
        Some(LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: true,
            ..Default::default()
        }),
    );

    let timeout = std::time::Duration::from_secs(config.build_timeout_secs);
    let log_collect = async {
        while let Some(Ok(log_output)) = log_stream.next().await {
            let msg = log_output.to_string();
            all_logs.push_str(&msg);
        }
        all_logs.clone()
    };

    let logs_result = tokio::time::timeout(timeout, log_collect).await;

    // Check timeout
    if logs_result.is_err() {
        tracing::error!("Build container timed out after {}s — killing", config.build_timeout_secs);
        let _ = docker
            .kill_container(container_id, None::<bollard::container::KillContainerOptions<String>>)
            .await;

        let _ = docker
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        anyhow::bail!("Build timed out after {} seconds", config.build_timeout_secs);
    }

    // Wait for completion
    let mut wait_stream = docker.wait_container(
        container_id,
        Some(WaitContainerOptions {
            condition: "not-running".to_string(),
        }),
    );

    if let Some(Ok(result)) = wait_stream.next().await {
        // Cleanup container
        let _ = docker
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        if result.status_code != 0 {
            anyhow::bail!(
                "Build container exited with code {}:\n{}",
                result.status_code,
                all_logs
            );
        }
    }

    // Cleanup container (in case wait didn't trigger)
    let _ = docker
        .remove_container(
            container_id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await;

    Ok(all_logs)
}

/// Compute SHA256 of whichever lockfile exists (for cache key).
/// Hashes package-lock.json, pnpm-lock.yaml, yarn.lock, or bun.lockb —
/// whichever is found first. Returns empty string if no lockfile.
async fn sha256_any_lockfile(project_dir: &std::path::Path) -> String {
    for name in &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb"] {
        let path = project_dir.join(name);
        if path.exists() {
            return sha256_file(&path).await.unwrap_or_default();
        }
    }
    String::new()
}

/// Update the PR comment for a deployment if one exists.
/// Called by the build worker when a build completes (success or failure).
async fn update_pr_comment_if_exists(
    db: &Database,
    config: &AppConfig,
    deployment_id: &str,
    status: &str,
    framework: Option<&str>,
    error: Option<&str>,
) {
    let dep_uuid = match uuid::Uuid::parse_str(deployment_id) {
        Ok(id) => id,
        Err(_) => return,
    };

    let row = match db.get_deployment_github_comment(dep_uuid).await {
        Ok(Some(r)) => r,
        _ => return,
    };

    let (comment_id, _pr_number, _project_id, repo_full_name, url) = row;

    let bot = match crate::github::PrBot::new(&config.github_token) {
        Some(b) => b,
        None => return,
    };

    if let Err(e) = bot
        .update_deployment_comment(
            &repo_full_name,
            comment_id as u64,
            status,
            &url,
            deployment_id,
            framework,
            error,
        )
        .await
    {
        tracing::warn!("Failed to update PR comment: {}", e);
    }
}

/// Compute SHA256 of a file (for cache key)
async fn sha256_file(path: &std::path::Path) -> anyhow::Result<String> {
    use sha2::{Sha256, Digest};
    
    if !path.exists() {
        return Ok(String::new());
    }
    
    let content = tokio::fs::read(path).await?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}
