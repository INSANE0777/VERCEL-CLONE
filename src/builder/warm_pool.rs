use super::firecracker::{FirecrackerRunner, MicroVm};
use super::framework::Framework;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

/// A warm pool of pre-booted Firecracker microVMs.
/// Cuts build start latency from ~3-5s (cold boot) to ~100-200ms (grab idle VM).
///
/// Each VM is ephemeral — acquired for one build, then destroyed.
/// A background task continuously replenishes the pool.
pub struct WarmPool {
    runner: FirecrackerRunner,
    target_size: usize,
    /// Idle VMs ready to accept a build
    idle: tokio::sync::Mutex<Vec<MicroVm>>,
    /// VMs currently running builds
    active_count: Arc<AtomicUsize>,
    /// Signal to stop the replenishment task
    shutdown: Arc<AtomicBool>,
}

impl WarmPool {
    /// Create a new warm pool. Does not start spawning VMs yet — call `start()`.
    pub fn new(runner: FirecrackerRunner, target_size: usize) -> Arc<Self> {
        Arc::new(Self {
            runner,
            target_size,
            idle: tokio::sync::Mutex::new(Vec::with_capacity(target_size)),
            active_count: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Start the background replenishment task.
    /// Call once at system startup.
    pub fn start(self: Arc<Self>) {
        let pool = self.clone();
        tokio::spawn(async move {
            tracing::info!("Warm pool replenishment started (target: {} VMs)", pool.target_size);
            loop {
                if pool.shutdown.load(Ordering::Relaxed) {
                    tracing::info!("Warm pool shutting down");
                    break;
                }

                let idle_count = pool.idle.lock().await.len();
                let active = pool.active_count.load(Ordering::Relaxed);
                let total = idle_count + active;

                if total < pool.target_size {
                    tracing::debug!(
                        "Warm pool low (idle={} active={} target={}) — spawning replacement",
                        idle_count, active, pool.target_size
                    );
                    match pool.runner.spawn_pool_vm(idle_count + active).await {
                        Ok(vm) => {
                            let id = vm.id.clone();
                            pool.idle.lock().await.push(vm);
                            tracing::info!("Warm pool VM {} ready (idle={})", id, idle_count + 1);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to spawn warm pool VM: {}", e);
                            sleep(Duration::from_secs(2)).await;
                        }
                    }
                } else {
                    // Pool is full — sleep and check again
                    sleep(Duration::from_millis(500)).await;
                }
            }

            // Shutdown: destroy all idle VMs
            let mut idle = pool.idle.lock().await;
            for vm in idle.drain(..) {
                let _ = pool.runner.destroy_vm(vm).await;
            }
        });
    }

    /// Grab an idle VM for a build. Returns None if pool is empty (caller should cold-boot).
    pub async fn acquire(&self) -> Option<MicroVm> {
        let mut idle = self.idle.lock().await;
        if let Some(vm) = idle.pop() {
            self.active_count.fetch_add(1, Ordering::SeqCst);
            tracing::info!(
                "Acquired warm pool VM {} (idle remaining: {})",
                vm.id,
                idle.len()
            );
            Some(vm)
        } else {
            tracing::warn!("Warm pool empty — falling back to cold boot");
            None
        }
    }

    /// Return a VM after build completion. VM is destroyed and replacement spawned in background.
    pub async fn release(&self, vm: MicroVm) {
        let id = vm.id.clone();
        if let Err(e) = self.runner.destroy_vm(vm).await {
            tracing::warn!("Failed to destroy warm pool VM {}: {}", id, e);
        }
        self.active_count.fetch_sub(1, Ordering::SeqCst);
        tracing::debug!("Released warm pool VM {} (background spawn triggered)", id);
        // The background replenishment task will notice the gap and spawn a replacement
    }

    /// Run a complete build inside a warm pool VM.
    /// Clones repo inside VM, runs build, copies output back.
    pub async fn run_build(
        &self,
        vm: &MicroVm,
        repo_url: &str,
        sha: &str,
        fw: &Framework,
        env_vars: &[(String, String)],
        host_build_dir: &Path,
    ) -> anyhow::Result<String> {
        // Build script to run inside VM
        let env_exports = env_vars
            .iter()
            .map(|(k, v)| format!("export {}='{}'", k, v.replace('\'', "'\"'\"'")))
            .collect::<Vec<_>>()
            .join(" && ");

        let build_script = format!(
            "mkdir -p /project && cd /project && \
             git clone --depth 1 {repo_url} . && \
             {sha_cmd} && \
             {install} && \
             {build}",
            repo_url = repo_url,
            sha_cmd = if sha != "HEAD" {
                format!("git fetch --depth 1 origin {} && git checkout {}", sha, sha)
            } else {
                "echo 'using HEAD'".into()
            },
            install = fw.install_command,
            build = fw.build_command,
        );

        let full_cmd = if env_exports.is_empty() {
            build_script
        } else {
            format!("{} && {}", env_exports, build_script)
        };

        tracing::info!(
            "Running build in warm pool VM {}: {} ...",
            vm.id,
            &full_cmd[..full_cmd.len().min(120)]
        );

        // Execute via SSH
        let mut child = Command::new("ssh")
            .args([
                "-i",
                self.runner.ssh_key_path.to_str().unwrap(),
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "UserKnownHostsFile=/dev/null",
                &format!("root@{}", vm.vm_ip),
                &full_cmd,
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Stream logs
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut stdout_reader = tokio::io::AsyncBufReadExt::lines(tokio::io::BufReader::new(stdout));
        let mut stderr_reader = tokio::io::AsyncBufReadExt::lines(tokio::io::BufReader::new(stderr));

        let mut all_logs = String::new();

        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            tracing::debug!("[warm-pool stdout] {}", l);
                            all_logs.push_str(&l);
                            all_logs.push('\n');
                        }
                        Ok(None) => break,
                        Err(e) => { tracing::warn!("stdout error: {}", e); break; }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            tracing::debug!("[warm-pool stderr] {}", l);
                            all_logs.push_str(&l);
                            all_logs.push('\n');
                        }
                        Ok(None) => break,
                        Err(e) => { tracing::warn!("stderr error: {}", e); break; }
                    }
                }
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!(
                "Build in warm pool VM {} failed (exit: {:?}):\n{}",
                vm.id,
                status.code(),
                all_logs
            );
        }

        // Copy build output from VM back to host
        let scp_output = format!("/project/{}/.", fw.output_dir);
        let scp = Command::new("scp")
            .args([
                "-i",
                self.runner.ssh_key_path.to_str().unwrap(),
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "UserKnownHostsFile=/dev/null",
                "-r",
                &format!("root@{}:{}", vm.vm_ip, scp_output),
                &format!("{}/", host_build_dir.display()),
            ])
            .output()
            .await?;

        if !scp.status.success() {
            // Output dir might not exist — try alternatives
            for alt_dir in &["out", "build", "dist", "public", "_site"] {
                let alt_scp = Command::new("scp")
                    .args([
                        "-i",
                        self.runner.ssh_key_path.to_str().unwrap(),
                        "-o",
                        "StrictHostKeyChecking=no",
                        "-o",
                        "UserKnownHostsFile=/dev/null",
                        "-r",
                        &format!("root@{}:/project/{}/.", vm.vm_ip, alt_dir),
                        &format!("{}/", host_build_dir.display()),
                    ])
                    .output()
                    .await?;
                if alt_scp.status.success() {
                    break;
                }
            }
        }

        tracing::info!("Build completed in warm pool VM {} — {} bytes of logs", vm.id, all_logs.len());
        Ok(all_logs)
    }

    /// Signal the pool to shut down (destroys all idle VMs).
    pub async fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        sleep(Duration::from_millis(100)).await; // Let task notice
    }
}
