#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

// ── Platform-conditional compilation ──────────────────────────────
// Firecracker requires Linux + KVM. On non-Unix platforms (Windows),
// this module compiles to stubs that always report unavailable.

#[cfg(unix)]
mod unix_impl {
    pub use tokio::net::UnixStream;
    pub use std::os::unix::fs::PermissionsExt;
}

#[cfg(not(unix))]
mod unix_impl {
    // Stub types for non-Unix compilation
    pub struct UnixStream;
    impl UnixStream {
        pub async fn connect(_: &std::path::Path) -> std::io::Result<Self> {
            Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Unix sockets not available"))
        }
    }
    pub trait PermissionsExt {
        fn mode(&self) -> u32;
    }
    impl PermissionsExt for std::fs::Permissions {
        fn mode(&self) -> u32 { 0 }
    }
}

use unix_impl::*;

/// Firecracker microVM build runner.
/// Provides hardware-enforced kernel isolation for builds — like Vercel's Hive.
/// On non-Linux systems, always reports unavailable and falls back to Docker.
#[derive(Clone)]
pub struct FirecrackerRunner {
    pub kernel_path: PathBuf,
    pub base_rootfs_path: PathBuf,
    pub socket_dir: PathBuf,
    pub tap_base: String,
    pub vm_ip_base: String,
    pub ssh_key_path: PathBuf,
    pub memory_mib: i64,
    pub vcpu_count: i64,
}

/// A running microVM instance
pub struct MicroVm {
    pub id: String,
    pub socket_path: PathBuf,
    pub tap_dev: String,
    pub vm_ip: String,
    pub pid: Option<u32>,
    pub rootfs_path: PathBuf,
}

impl FirecrackerRunner {
    /// Check if Firecracker can be used on this system.
    /// Requires: Linux, /dev/kvm exists, firecracker binary in PATH.
    pub async fn is_available() -> bool {
        #[cfg(not(unix))]
        {
            tracing::info!("Firecracker unavailable: requires Linux with KVM (not a Unix system)");
            return false;
        }

        #[cfg(unix)]
        {
            // Check /dev/kvm
            if !Path::new("/dev/kvm").exists() {
                tracing::info!("Firecracker unavailable: /dev/kvm not found (need Linux with KVM)");
                return false;
            }

            // Check firecracker binary
            match Command::new("which").arg("firecracker").output().await {
                Ok(output) if output.status.success() => {
                    tracing::info!("Firecracker binary found");
                }
                _ => {
                    tracing::info!("Firecracker unavailable: 'firecracker' binary not in PATH");
                    return false;
                }
            }

            // Check KVM permissions
            match tokio::fs::metadata("/dev/kvm").await {
                Ok(meta) => {
                    #[cfg(unix)]
                    {
                        tracing::info!("/dev/kvm exists (mode: {:o})", meta.permissions().mode());
                    }
                    #[cfg(not(unix))]
                    {
                        tracing::info!("/dev/kvm exists");
                    }
                }
                Err(e) => {
                    tracing::warn!("Cannot access /dev/kvm: {}", e);
                    return false;
                }
            }

            tracing::info!("Firecracker microVMs are AVAILABLE on this system");
            true
        }
    }

    /// Create a new runner with default paths.
    pub fn new(data_dir: &Path) -> Self {
        let fc_dir = data_dir.join("firecracker");
        Self {
            kernel_path: fc_dir.join("vmlinux"),
            base_rootfs_path: fc_dir.join("rootfs.ext4"),
            socket_dir: fc_dir.join("sockets"),
            tap_base: "fc-tap".into(),
            vm_ip_base: "172.16.0".into(),
            ssh_key_path: fc_dir.join("vm_key"),
            memory_mib: 2048,
            vcpu_count: 2,
        }
    }

    /// Prepare the Firecracker data directory (kernel, rootfs, SSH keys).
    pub async fn prepare(&self, docker_image: &str) -> anyhow::Result<()> {
        #[cfg(not(unix))]
        {
            anyhow::bail!("Firecracker requires Linux with KVM");
        }

        #[cfg(unix)]
        {
            tokio::fs::create_dir_all(&self.socket_dir).await?;

            // 1. Prepare rootfs from Docker image if not exists
            if !self.base_rootfs_path.exists() {
                tracing::info!("Preparing Firecracker rootfs from Docker image: {}", docker_image);
                self.create_rootfs_from_docker(docker_image).await?;
            }

            // 2. Download kernel if not exists
            if !self.kernel_path.exists() {
                tracing::info!("Downloading Firecracker kernel...");
                self.download_kernel().await?;
            }

            // 3. Generate SSH key pair for VM access
            if !self.ssh_key_path.exists() {
                tracing::info!("Generating SSH key for VM access...");
                self.generate_ssh_key().await?;
            }

            tracing::info!("Firecracker runner prepared and ready");
            Ok(())
        }
    }

    /// Create a bootable ext4 rootfs from a Docker image.
    #[cfg(unix)]
    async fn create_rootfs_from_docker(&self, image: &str) -> anyhow::Result<()> {
        let work_dir = self.base_rootfs_path.parent().unwrap().join("rootfs-work");
        tokio::fs::create_dir_all(&work_dir).await?;

        // Export Docker image to tarball
        tracing::info!("Exporting Docker image {}...", image);
        let export = Command::new("docker")
            .args(["create", image])
            .output()
            .await?;
        
        if !export.status.success() {
            anyhow::bail!("Failed to create container from image: {}", String::from_utf8_lossy(&export.stderr));
        }
        
        let container_id = String::from_utf8_lossy(&export.stdout).trim().to_string();
        
        let tar_path = work_dir.join("rootfs.tar");
        let export_tar = Command::new("docker")
            .args(["export", "-o", tar_path.to_str().unwrap(), &container_id])
            .output()
            .await?;
        
        if !export_tar.status.success() {
            let _ = Command::new("docker").args(["rm", &container_id]).output().await;
            anyhow::bail!("Failed to export container: {}", String::from_utf8_lossy(&export_tar.stderr));
        }
        
        let _ = Command::new("docker").args(["rm", &container_id]).output().await;

        // Create ext4 image (3GB)
        tracing::info!("Creating ext4 rootfs image (3GB)...");
        let dd = Command::new("dd")
            .args(["if=/dev/zero", 
                   &format!("of={}", self.base_rootfs_path.display()),
                   "bs=1M", "count=3072"])
            .output()
            .await?;
        
        if !dd.status.success() {
            anyhow::bail!("Failed to create rootfs file: {}", String::from_utf8_lossy(&dd.stderr));
        }

        // Format as ext4
        let mkfs = Command::new("mkfs.ext4")
            .arg("-F")
            .arg(self.base_rootfs_path.to_str().unwrap())
            .output()
            .await?;
        
        if !mkfs.status.success() {
            anyhow::bail!("Failed to format rootfs: {}", String::from_utf8_lossy(&mkfs.stderr));
        }

        // Mount and extract tarball
        let mount_dir = work_dir.join("mnt");
        tokio::fs::create_dir_all(&mount_dir).await?;
        
        let mount = Command::new("mount")
            .args(["-o", "loop", self.base_rootfs_path.to_str().unwrap(), mount_dir.to_str().unwrap()])
            .output()
            .await?;
        
        if !mount.status.success() {
            anyhow::bail!("Failed to mount rootfs: {}", String::from_utf8_lossy(&mount.stderr));
        }

        // Extract tarball
        let extract = Command::new("tar")
            .args(["-xf", tar_path.to_str().unwrap(), "-C", mount_dir.to_str().unwrap()])
            .output()
            .await?;
        
        if !extract.status.success() {
            let _ = Command::new("umount").arg(mount_dir.to_str().unwrap()).output().await;
            anyhow::bail!("Failed to extract rootfs: {}", String::from_utf8_lossy(&extract.stderr));
        }

        // Add init script and SSH setup
        self.setup_vm_init(&mount_dir).await?;

        // Unmount
        let umount = Command::new("umount")
            .arg(mount_dir.to_str().unwrap())
            .output()
            .await?;
        
        if !umount.status.success() {
            tracing::warn!("Failed to unmount rootfs: {}", String::from_utf8_lossy(&umount.stderr));
        }

        // Cleanup
        let _ = tokio::fs::remove_dir_all(&work_dir).await;
        
        tracing::info!("Rootfs prepared at {}", self.base_rootfs_path.display());
        Ok(())
    }

    #[cfg(not(unix))]
    async fn create_rootfs_from_docker(&self, _image: &str) -> anyhow::Result<()> {
        anyhow::bail!("Rootfs creation requires Linux")
    }

    /// Set up init system inside the rootfs for VM boot.
    #[cfg(unix)]
    async fn setup_vm_init(&self, mount_dir: &Path) -> anyhow::Result<()> {
        let init_script = r#"#!/bin/sh
# Firecracker microVM init
mount -t proc proc /proc
mount -t sysfs sys /sys
mount -t devtmpfs dev /dev

# Network setup (DHCP or static)
ip link set eth0 up
ip addr add 172.16.0.2/24 dev eth0
ip route add default via 172.16.0.1

# Start SSH daemon
mkdir -p /run/sshd
/usr/sbin/sshd -D &

# Keep init alive
while true; do sleep 3600; done
"#;

        let init_path = mount_dir.join("init.sh");
        tokio::fs::write(&init_path, init_script).await?;
        
        let chmod = Command::new("chmod")
            .args(["+x", init_path.to_str().unwrap()])
            .output()
            .await?;
        
        if !chmod.status.success() {
            tracing::warn!("Failed to chmod init script");
        }

        let ssh_dir = mount_dir.join("root/.ssh");
        tokio::fs::create_dir_all(&ssh_dir).await?;
        
        Ok(())
    }

    #[cfg(not(unix))]
    async fn setup_vm_init(&self, _mount_dir: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    /// Download a compatible kernel for Firecracker.
    #[cfg(unix)]
    async fn download_kernel(&self) -> anyhow::Result<()> {
        let kernel_url = "https://s3.amazonaws.com/spec.ccfc.min/firecracker-ci/v1.5/x86_64/vmlinux-5.10";
        
        let client = reqwest::Client::new();
        let response = client.get(kernel_url).send().await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to download kernel: HTTP {}", response.status());
        }
        
        let bytes = response.bytes().await?;
        tokio::fs::write(&self.kernel_path, bytes).await?;
        
        tracing::info!("Kernel downloaded to {}", self.kernel_path.display());
        Ok(())
    }

    #[cfg(not(unix))]
    async fn download_kernel(&self) -> anyhow::Result<()> {
        anyhow::bail!("Kernel download requires Linux")
    }

    /// Generate SSH key pair for VM access.
    #[cfg(unix)]
    async fn generate_ssh_key(&self) -> anyhow::Result<()> {
        let keygen = Command::new("ssh-keygen")
            .args([
                "-t", "ed25519",
                "-f", self.ssh_key_path.to_str().unwrap(),
                "-N", "",
                "-C", "vercel-clone-vm"
            ])
            .output()
            .await?;
        
        if !keygen.status.success() {
            anyhow::bail!("Failed to generate SSH key: {}", String::from_utf8_lossy(&keygen.stderr));
        }

        // Add public key to rootfs authorized_keys if rootfs exists
        let pub_key_path = self.ssh_key_path.with_extension("pub");
        if self.base_rootfs_path.exists() && pub_key_path.exists() {
            let pub_key = tokio::fs::read_to_string(&pub_key_path).await?;
            
            let work_dir = self.base_rootfs_path.parent().unwrap().join("ssh-setup");
            tokio::fs::create_dir_all(&work_dir).await?;
            
            let mount = Command::new("mount")
                .args(["-o", "loop", self.base_rootfs_path.to_str().unwrap(), work_dir.to_str().unwrap()])
                .output()
                .await?;
            
            if mount.status.success() {
                let auth_keys = work_dir.join("root/.ssh/authorized_keys");
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&auth_keys)
                    .await?;
                file.write_all(pub_key.trim().as_bytes()).await?;
                file.write_all(b"\n").await?;
                drop(file);
                
                let _ = Command::new("umount").arg(work_dir.to_str().unwrap()).output().await;
            }
        }

        tracing::info!("SSH key generated at {}", self.ssh_key_path.display());
        Ok(())
    }

    #[cfg(not(unix))]
    async fn generate_ssh_key(&self) -> anyhow::Result<()> {
        anyhow::bail!("SSH key generation requires Linux")
    }

    /// Spawn a new microVM for a build.
    pub async fn spawn_vm(&self, project_dir: &Path) -> anyhow::Result<MicroVm> {
        #[cfg(not(unix))]
        {
            anyhow::bail!("Firecracker microVMs require Linux with KVM")
        }

        #[cfg(unix)]
        {
            let vm_id = format!("vm-{}", Uuid::new_v4());
            let socket_path = self.socket_dir.join(format!("{}.sock", vm_id));
            let tap_dev = format!("{}-{}", self.tap_base, &vm_id[..8]);
            let vm_ip = format!("{}.2", self.vm_ip_base); // Host is .1
            
            // Create a copy-on-write rootfs for this VM
            let vm_rootfs = self.socket_dir.join(format!("{}-rootfs.ext4", vm_id));
            
            let copy = Command::new("cp")
                .args(["--sparse=always",
                       self.base_rootfs_path.to_str().unwrap(),
                       vm_rootfs.to_str().unwrap()])
                .output()
                .await?;
            
            if !copy.status.success() {
                anyhow::bail!("Failed to copy rootfs: {}", String::from_utf8_lossy(&copy.stderr));
            }

            // Create project disk (second drive with project files)
            let project_disk = self.socket_dir.join(format!("{}-project.ext4", vm_id));
            self.create_project_disk(project_dir, &project_disk).await?;

            // Setup TAP interface
            self.setup_tap(&tap_dev).await?;

            // Start Firecracker process
            let child = Command::new("firecracker")
                .args([
                    "--api-sock", socket_path.to_str().unwrap(),
                    "--id", &vm_id,
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let pid = child.id();
            
            // Wait a moment for Firecracker to start listening
            sleep(Duration::from_millis(500)).await;

            // Configure VM via HTTP API over Unix socket
            self.configure_vm(&socket_path, &vm_rootfs, &project_disk, &tap_dev).await?;

            // Start the VM
            self.start_vm(&socket_path).await?;

            // Wait for VM to boot (SSH available)
            self.wait_for_vm_ready(&vm_ip).await?;

            tracing::info!("MicroVM {} started (PID: {:?}, IP: {})", vm_id, pid, vm_ip);

            Ok(MicroVm {
                id: vm_id,
                socket_path,
                tap_dev,
                vm_ip,
                pid,
                rootfs_path: vm_rootfs,
            })
        }
    }

    /// Spawn a lightweight microVM for the warm pool (no project disk, just base rootfs + SSH).
    /// Uses indexed IPs/TAPs so multiple pool VMs can coexist.
    pub async fn spawn_pool_vm(&self, pool_index: usize) -> anyhow::Result<MicroVm> {
        #[cfg(not(unix))]
        {
            anyhow::bail!("Firecracker microVMs require Linux with KVM")
        }

        #[cfg(unix)]
        {
            let vm_id = format!("vm-pool-{}", pool_index);
            let socket_path = self.socket_dir.join(format!("{}.sock", vm_id));
            let tap_dev = format!("{}-pool-{}", self.tap_base, pool_index);
            // Use 172.16.0.10+ for pool VMs to avoid conflict with ad-hoc VMs at .2
            let vm_ip = format!("{}.{}", self.vm_ip_base, 10 + pool_index);
            let guest_mac = format!("AA:FC:00:00:00:{:02X}", 10 + pool_index);
            
            // Create a copy-on-write rootfs for this VM
            let vm_rootfs = self.socket_dir.join(format!("{}-rootfs.ext4", vm_id));
            
            let copy = Command::new("cp")
                .args(["--sparse=always",
                       self.base_rootfs_path.to_str().unwrap(),
                       vm_rootfs.to_str().unwrap()])
                .output()
                .await?;
            
            if !copy.status.success() {
                anyhow::bail!("Failed to copy rootfs: {}", String::from_utf8_lossy(&copy.stderr));
            }

            // Setup TAP interface (unique per pool index)
            self.setup_tap_pool(&tap_dev, &vm_ip).await?;

            // Start Firecracker process
            let child = Command::new("firecracker")
                .args([
                    "--api-sock", socket_path.to_str().unwrap(),
                    "--id", &vm_id,
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let pid = child.id();
            
            sleep(Duration::from_millis(500)).await;

            // Configure VM — no project disk, just rootfs + network
            self.configure_pool_vm(&socket_path, &vm_rootfs, &tap_dev, &guest_mac, &vm_ip).await?;
            self.start_vm(&socket_path).await?;
            self.wait_for_vm_ready(&vm_ip).await?;

            tracing::info!("Pool microVM {} started (PID: {:?}, IP: {})", vm_id, pid, vm_ip);

            Ok(MicroVm {
                id: vm_id,
                socket_path,
                tap_dev,
                vm_ip,
                pid,
                rootfs_path: vm_rootfs,
            })
        }
    }

    /// Setup TAP interface for a pool VM with a specific IP.
    #[cfg(unix)]
    async fn setup_tap_pool(&self, tap_dev: &str, vm_ip: &str) -> anyhow::Result<()> {
        // Extract last octet from vm_ip
        let last_octet = vm_ip.split('.').last().unwrap_or("10");
        let host_ip = format!("{}.1", self.vm_ip_base);

        let ip = Command::new("ip")
            .args(["tuntap", "add", tap_dev, "mode", "tap"])
            .output()
            .await?;
        
        if !ip.status.success() {
            tracing::debug!("TAP creation output: {}", String::from_utf8_lossy(&ip.stderr));
        }

        let up = Command::new("ip")
            .args(["link", "set", tap_dev, "up"])
            .output()
            .await?;
        
        if !up.status.success() {
            tracing::warn!("Failed to bring up TAP: {}", String::from_utf8_lossy(&up.stderr));
        }

        // Assign IP on a /24 subnet — each pool VM gets its own /24 to avoid ARP conflicts
        // Actually they share the same subnet, just different IPs
        let addr = Command::new("ip")
            .args(["addr", "add", &format!("{}/24", host_ip), "dev", tap_dev])
            .output()
            .await?;
        
        if !addr.status.success() {
            tracing::debug!("IP assignment output: {}", String::from_utf8_lossy(&addr.stderr));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    async fn setup_tap_pool(&self, _tap_dev: &str, _vm_ip: &str) -> anyhow::Result<()> {
        Ok(())
    }

    /// Configure a pool VM — rootfs + network only (no project drive).
    #[cfg(unix)]
    async fn configure_pool_vm(
        &self,
        socket_path: &Path,
        rootfs_path: &Path,
        tap_dev: &str,
        guest_mac: &str,
        vm_ip: &str,
    ) -> anyhow::Result<()> {
        let machine_config = serde_json::json!({
            "vcpu_count": self.vcpu_count,
            "mem_size_mib": self.memory_mib,
            "track_dirty_pages": false,
        });
        self.api_put(socket_path, "/machine-config", &machine_config).await?;

        // Extract last octet for boot args
        let last_octet = vm_ip.split('.').last().unwrap_or("10");
        let host_ip = format!("{}.1", self.vm_ip_base);

        let boot_source = serde_json::json!({
            "kernel_image_path": self.kernel_path.to_str().unwrap(),
            "boot_args": format!(
                "console=ttyS0 reboot=k panic=1 pci=off \
                 ip={}::{}:{}::eth0:off \
                 init=/init.sh",
                last_octet, host_ip, host_ip
            ),
        });
        self.api_put(socket_path, "/boot-source", &boot_source).await?;

        let rootfs_drive = serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": rootfs_path.to_str().unwrap(),
            "is_root_device": true,
            "is_read_only": false,
        });
        self.api_put(socket_path, "/drives/rootfs", &rootfs_drive).await?;

        let net_iface = serde_json::json!({
            "iface_id": "eth0",
            "guest_mac": guest_mac,
            "host_dev_name": tap_dev,
        });
        self.api_put(socket_path, "/network-interfaces/eth0", &net_iface).await?;

        Ok(())
    }

    #[cfg(not(unix))]
    async fn configure_pool_vm(&self, _: &Path, _: &Path, _: &str, _: &str, _: &str) -> anyhow::Result<()> {
        Ok(())
    }

    /// Create a small ext4 disk with project files.
    #[cfg(unix)]
    async fn create_project_disk(&self, project_dir: &Path, disk_path: &Path) -> anyhow::Result<()> {
        let du = Command::new("du")
            .args(["-sb", project_dir.to_str().unwrap()])
            .output()
            .await?;
        
        let size_bytes: u64 = String::from_utf8_lossy(&du.stdout)
            .split_whitespace()
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1024 * 1024);
        
        let size_mb = ((size_bytes + 512 * 1024 * 1024) / (1024 * 1024)).max(256);

        let dd = Command::new("dd")
            .args(["if=/dev/zero",
                   &format!("of={}", disk_path.display()),
                   &format!("bs=1M"),
                   &format!("count={}", size_mb)])
            .output()
            .await?;
        
        if !dd.status.success() {
            anyhow::bail!("Failed to create project disk");
        }

        let mkfs = Command::new("mkfs.ext4")
            .arg("-F")
            .arg(disk_path.to_str().unwrap())
            .output()
            .await?;
        
        if !mkfs.status.success() {
            anyhow::bail!("Failed to format project disk");
        }

        let mount_dir = disk_path.parent().unwrap().join(format!("mnt-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&mount_dir).await?;
        
        let mount = Command::new("mount")
            .args(["-o", "loop", disk_path.to_str().unwrap(), mount_dir.to_str().unwrap()])
            .output()
            .await?;
        
        if !mount.status.success() {
            anyhow::bail!("Failed to mount project disk");
        }

        let cp = Command::new("cp")
            .args(["-a",
                   &format!("{}/.", project_dir.display()),
                   &format!("{}/", mount_dir.display())])
            .output()
            .await?;
        
        if !cp.status.success() {
            let _ = Command::new("umount").arg(mount_dir.to_str().unwrap()).output().await;
            anyhow::bail!("Failed to copy project to disk");
        }

        let umount = Command::new("umount")
            .arg(mount_dir.to_str().unwrap())
            .output()
            .await?;
        
        if !umount.status.success() {
            tracing::warn!("Failed to unmount project disk");
        }

        let _ = tokio::fs::remove_dir(&mount_dir).await;
        
        tracing::info!("Project disk created at {} ({}MB)", disk_path.display(), size_mb);
        Ok(())
    }

    #[cfg(not(unix))]
    async fn create_project_disk(&self, _project_dir: &Path, _disk_path: &Path) -> anyhow::Result<()> {
        anyhow::bail!("Project disk creation requires Linux")
    }

    /// Setup TAP network interface for VM communication.
    #[cfg(unix)]
    async fn setup_tap(&self, tap_dev: &str) -> anyhow::Result<()> {
        let ip = Command::new("ip")
            .args(["tuntap", "add", tap_dev, "mode", "tap"])
            .output()
            .await?;
        
        if !ip.status.success() {
            tracing::debug!("TAP device creation output: {}", String::from_utf8_lossy(&ip.stderr));
        }

        let up = Command::new("ip")
            .args(["link", "set", tap_dev, "up"])
            .output()
            .await?;
        
        if !up.status.success() {
            tracing::warn!("Failed to bring up TAP: {}", String::from_utf8_lossy(&up.stderr));
        }

        let addr = Command::new("ip")
            .args(["addr", "add", &format!("{}/24", self.host_ip()), "dev", tap_dev])
            .output()
            .await?;
        
        if !addr.status.success() {
            tracing::debug!("IP assignment output: {}", String::from_utf8_lossy(&addr.stderr));
        }

        let _ = Command::new("sysctl").args(["-w", "net.ipv4.ip_forward=1"]).output().await;
        
        Ok(())
    }

    #[cfg(not(unix))]
    async fn setup_tap(&self, _tap_dev: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn host_ip(&self) -> String {
        format!("{}.1", self.vm_ip_base)
    }

    /// Configure VM via Firecracker HTTP API.
    #[cfg(unix)]
    async fn configure_vm(
        &self,
        socket_path: &Path,
        rootfs_path: &Path,
        project_disk: &Path,
        tap_dev: &str,
    ) -> anyhow::Result<()> {
        let machine_config = serde_json::json!({
            "vcpu_count": self.vcpu_count,
            "mem_size_mib": self.memory_mib,
            "track_dirty_pages": false,
        });
        self.api_put(socket_path, "/machine-config", &machine_config).await?;

        let boot_source = serde_json::json!({
            "kernel_image_path": self.kernel_path.to_str().unwrap(),
            "boot_args": format!(
                "console=ttyS0 reboot=k panic=1 pci=off \
                 ip={}::{}:{}::eth0:off \
                 init=/init.sh",
                self.vm_ip_base.split('.').nth(3).unwrap_or("2"),
                self.host_ip(),
                self.host_ip()
            ),
        });
        self.api_put(socket_path, "/boot-source", &boot_source).await?;

        let rootfs_drive = serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": rootfs_path.to_str().unwrap(),
            "is_root_device": true,
            "is_read_only": false,
        });
        self.api_put(socket_path, "/drives/rootfs", &rootfs_drive).await?;

        let project_drive = serde_json::json!({
            "drive_id": "project",
            "path_on_host": project_disk.to_str().unwrap(),
            "is_root_device": false,
            "is_read_only": false,
        });
        self.api_put(socket_path, "/drives/project", &project_drive).await?;

        let net_iface = serde_json::json!({
            "iface_id": "eth0",
            "guest_mac": "AA:FC:00:00:00:01",
            "host_dev_name": tap_dev,
        });
        self.api_put(socket_path, "/network-interfaces/eth0", &net_iface).await?;

        Ok(())
    }

    #[cfg(not(unix))]
    async fn configure_vm(&self, _: &Path, _: &Path, _: &Path, _: &str) -> anyhow::Result<()> {
        Ok(())
    }

    /// Start the VM.
    #[cfg(unix)]
    async fn start_vm(&self, socket_path: &Path) -> anyhow::Result<()> {
        let action = serde_json::json!({
            "action_type": "InstanceStart",
        });
        self.api_put(socket_path, "/actions", &action).await?;
        Ok(())
    }

    #[cfg(not(unix))]
    async fn start_vm(&self, _: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    /// Simple HTTP-over-Unix-socket client for Firecracker API.
    #[cfg(unix)]
    async fn api_put(&self, socket_path: &Path, path: &str, body: &serde_json::Value) -> anyhow::Result<()> {
        let body_str = body.to_string();
        let request = format!(
            "PUT {} HTTP/1.1\r\n\
             Host: localhost\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            path,
            body_str.len(),
            body_str
        );

        let mut stream = UnixStream::connect(socket_path).await?;
        stream.write_all(request.as_bytes()).await?;
        
        let mut buf = [0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await?;
        let response = String::from_utf8_lossy(&buf[..n]);
        
        if !response.contains("204") && !response.contains("200") {
            anyhow::bail!("Firecracker API error: {}", response);
        }

        Ok(())
    }

    #[cfg(not(unix))]
    async fn api_put(&self, _: &Path, _: &str, _: &serde_json::Value) -> anyhow::Result<()> {
        Ok(())
    }

    /// Wait for VM to be ready (SSH responsive).
    #[cfg(unix)]
    async fn wait_for_vm_ready(&self, vm_ip: &str) -> anyhow::Result<()> {
        let max_attempts = 60;
        for i in 0..max_attempts {
            let ssh_test = Command::new("ssh")
                .args([
                    "-i", self.ssh_key_path.to_str().unwrap(),
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "UserKnownHostsFile=/dev/null",
                    "-o", "ConnectTimeout=1",
                    &format!("root@{}", vm_ip),
                    "echo ready",
                ])
                .output()
                .await?;
            
            if ssh_test.status.success() {
                tracing::info!("VM ready after {} attempts", i + 1);
                return Ok(());
            }
            
            sleep(Duration::from_millis(500)).await;
        }
        
        anyhow::bail!("VM failed to become ready after {} attempts", max_attempts)
    }

    #[cfg(not(unix))]
    async fn wait_for_vm_ready(&self, _: &str) -> anyhow::Result<()> {
        anyhow::bail!("VM boot requires Linux")
    }

    /// Run a build command inside the microVM via SSH.
    pub async fn run_build_in_vm(
        &self,
        vm: &MicroVm,
        framework: &super::framework::Framework,
        env_vars: &[(String, String)],
    ) -> anyhow::Result<String> {
        #[cfg(not(unix))]
        {
            anyhow::bail!("SSH build execution requires Linux")
        }

        #[cfg(unix)]
        {
            let mount_cmd = Command::new("ssh")
                .args([
                    "-i", self.ssh_key_path.to_str().unwrap(),
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "UserKnownHostsFile=/dev/null",
                    &format!("root@{}", vm.vm_ip),
                    "mount /dev/vdb /project && mkdir -p /project && mount /dev/vdb /project 2>/dev/null || true",
                ])
                .output()
                .await?;
            
            if !mount_cmd.status.success() {
                tracing::warn!("Project mount warning: {}", String::from_utf8_lossy(&mount_cmd.stderr));
            }

            let env_str = env_vars.iter()
                .map(|(k, v)| format!("export {}='{}'", k, v.replace('\'', "'\"'\"'")))
                .collect::<Vec<_>>()
                .join(" && ");

            let build_script = format!(
                "cd /project && {} && {}",
                framework.install_command,
                framework.build_command
            );
            
            let full_cmd = if env_str.is_empty() {
                build_script
            } else {
                format!("{} && {}", env_str, build_script)
            };

            tracing::info!("Running build in microVM {}: {}", vm.id, full_cmd);

            let mut child = Command::new("ssh")
                .args([
                    "-i", self.ssh_key_path.to_str().unwrap(),
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "UserKnownHostsFile=/dev/null",
                    &format!("root@{}", vm.vm_ip),
                    &full_cmd,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            
            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();
            
            let mut all_logs = String::new();
            
            loop {
                tokio::select! {
                    line = stdout_reader.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                tracing::debug!("[VM stdout] {}", l);
                                all_logs.push_str(&l);
                                all_logs.push('\n');
                            }
                            Ok(None) => break,
                            Err(e) => {
                                tracing::warn!("Error reading stdout: {}", e);
                                break;
                            }
                        }
                    }
                    line = stderr_reader.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                tracing::debug!("[VM stderr] {}", l);
                                all_logs.push_str(&l);
                                all_logs.push('\n');
                            }
                            Ok(None) => break,
                            Err(e) => {
                                tracing::warn!("Error reading stderr: {}", e);
                                break;
                            }
                        }
                    }
                }
            }

            let status = child.wait().await?;
            
            if !status.success() {
                anyhow::bail!("Build in microVM failed (exit code: {:?}):\n{}", status.code(), all_logs);
            }

            Ok(all_logs)
        }
    }

    /// Copy build output from VM to host.
    pub async fn copy_build_output(
        &self,
        vm: &MicroVm,
        output_dir: &str,
        host_dest: &Path,
    ) -> anyhow::Result<()> {
        #[cfg(not(unix))]
        {
            anyhow::bail!("SCP requires Linux")
        }

        #[cfg(unix)]
        {
            tokio::fs::create_dir_all(host_dest).await?;
            
            let scp = Command::new("scp")
                .args([
                    "-i", self.ssh_key_path.to_str().unwrap(),
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "UserKnownHostsFile=/dev/null",
                    "-r",
                    &format!("root@{}:/project/{}/.", vm.vm_ip, output_dir),
                    &format!("{}/", host_dest.display()),
                ])
                .output()
                .await?;
            
            if !scp.status.success() {
                tracing::warn!("SCP build output warning: {}", String::from_utf8_lossy(&scp.stderr));
            }

            Ok(())
        }
    }

    /// Destroy a microVM and clean up resources.
    pub async fn destroy_vm(&self, vm: MicroVm) -> anyhow::Result<()> {
        #[cfg(not(unix))]
        {
            tracing::debug!("Destroy VM stub on non-Unix platform");
            Ok(())
        }

        #[cfg(unix)]
        {
            let _ = self.send_shutdown(&vm.socket_path).await;

            if let Some(pid) = vm.pid {
                let _ = Command::new("kill").arg(pid.to_string()).output().await;
            }

            sleep(Duration::from_millis(500)).await;

            let _ = tokio::fs::remove_file(&vm.socket_path).await;
            let _ = tokio::fs::remove_file(&vm.rootfs_path).await;

            let project_disk = vm.rootfs_path.with_file_name(
                format!("{}-project.ext4", vm.id)
            );
            let _ = tokio::fs::remove_file(&project_disk).await;

            let _ = Command::new("ip")
                .args(["tuntap", "del", &vm.tap_dev, "mode", "tap"])
                .output()
                .await;

            tracing::info!("MicroVM {} destroyed", vm.id);
            Ok(())
        }
    }

    #[cfg(unix)]
    async fn send_shutdown(&self, socket_path: &Path) -> anyhow::Result<()> {
        let action = serde_json::json!({
            "action_type": "SendCtrlAltDel",
        });
        self.api_put(socket_path, "/actions", &action).await?;
        Ok(())
    }

    #[cfg(not(unix))]
    async fn send_shutdown(&self, _: &Path) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Drop for MicroVm {
    fn drop(&mut self) {
        tracing::debug!("MicroVm {} dropping (cleanup may be needed)", self.id);
    }
}
