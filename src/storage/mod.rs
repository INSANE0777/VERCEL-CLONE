use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::Region;
use std::path::Path;
use walkdir::WalkDir;

/// S3-compatible object storage for build artifacts.
/// Falls back to local filesystem if S3 is unavailable.
#[derive(Clone)]
pub struct ArtifactStore {
    bucket: Option<Box<Bucket>>,
    local_dir: String,
    use_s3: bool,
}

impl ArtifactStore {
    pub fn new(
        endpoint: &str,
        bucket_name: &str,
        access_key: &str,
        secret_key: &str,
        region: &str,
        local_dir: &str,
    ) -> Self {
        // Try to connect to S3
        let bucket = match Credentials::new(Some(access_key), Some(secret_key), None, None, None) {
            Ok(creds) => {
                match Bucket::new(
                    bucket_name,
                    Region::Custom {
                        region: region.to_string(),
                        endpoint: endpoint.to_string(),
                    },
                    creds,
                ) {
                    Ok(b) => Some(b.with_path_style()),
                    Err(e) => {
                        tracing::warn!("Failed to create S3 bucket: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create S3 credentials: {}", e);
                None
            }
        };

        let use_s3 = bucket.is_some();

        if use_s3 {
            tracing::info!("Artifact store: S3 at {} (bucket: {})", endpoint, bucket_name);
        } else {
            tracing::warn!("Artifact store: S3 unavailable, using local filesystem at {}", local_dir);
            std::fs::create_dir_all(local_dir).expect("Failed to create artifacts directory");
        }

        Self {
            bucket,
            local_dir: local_dir.to_string(),
            use_s3,
        }
    }

    /// Upload a directory of build artifacts to storage
    pub async fn store_build_output(
        &self,
        deployment_id: &str,
        source_dir: &Path,
    ) -> anyhow::Result<u64> {
        // Always copy locally so Caddy can serve the files
        let local_bytes = self.copy_to_local(deployment_id, source_dir).await?;
        
        // Also upload to S3 if available (for durability / edge replication)
        if self.use_s3 {
            if let Err(e) = self.upload_to_s3(deployment_id, source_dir).await {
                tracing::warn!("S3 upload failed (non-fatal, local copy exists): {}", e);
            }
        }
        
        Ok(local_bytes)
    }

    /// Upload to S3
    async fn upload_to_s3(
        &self,
        deployment_id: &str,
        source_dir: &Path,
    ) -> anyhow::Result<u64> {
        let bucket = self.bucket.as_ref().unwrap();
        let mut total_bytes: u64 = 0;

        for entry in WalkDir::new(source_dir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let relative = entry.path().strip_prefix(source_dir)?;
            let key = format!("artifacts/{}/{}", deployment_id, relative.display());
            let content = tokio::fs::read(entry.path()).await?;
            let content_len = content.len() as u64;

            bucket
                .put_object(&key, &content)
                .await?;

            total_bytes += content_len;
        }

        tracing::info!(
            "Uploaded {} bytes to S3 for deployment {}",
            total_bytes,
            deployment_id
        );
        Ok(total_bytes)
    }

    /// Copy to local filesystem (fallback)
    async fn copy_to_local(
        &self,
        deployment_id: &str,
        source_dir: &Path,
    ) -> anyhow::Result<u64> {
        let dest = Path::new(&self.local_dir).join(deployment_id);

        if dest.exists() {
            tokio::fs::remove_dir_all(&dest).await?;
        }

        let total_bytes = Self::copy_dir_recursive(source_dir, &dest).await?;

        tracing::info!(
            "Stored {} bytes locally for deployment {}",
            total_bytes,
            deployment_id
        );
        Ok(total_bytes)
    }

    /// Upload a cache (e.g., node_modules) to storage
    pub async fn store_cache(
        &self,
        project_id: &str,
        cache_key: &str,
        source_dir: &Path,
    ) -> anyhow::Result<u64> {
        let path = format!("cache/{}/{}", project_id, cache_key);

        if self.use_s3 {
            let bucket = self.bucket.as_ref().unwrap();
            let mut total_bytes: u64 = 0;

            for entry in WalkDir::new(source_dir).into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }
                let relative = entry.path().strip_prefix(source_dir)?;
                let key = format!("{}/{}", path, relative.display());
                let content = tokio::fs::read(entry.path()).await?;
                total_bytes += content.len() as u64;
                bucket.put_object(&key, &content).await?;
            }
            Ok(total_bytes)
        } else {
            let dest = Path::new(&self.local_dir).join(&path);
            Self::copy_dir_recursive(source_dir, &dest).await
        }
    }

    /// Download a cache from storage
    pub async fn restore_cache(
        &self,
        project_id: &str,
        cache_key: &str,
        dest_dir: &Path,
    ) -> anyhow::Result<bool> {
        let path = format!("cache/{}/{}", project_id, cache_key);

        if self.use_s3 {
            let bucket = self.bucket.as_ref().unwrap();
            let results = bucket.list(path.clone(), Some("/".to_string())).await?;

            if results.is_empty() {
                return Ok(false);
            }

            for object_list in &results {
                for object in &object_list.contents {
                    let data = bucket.get_object(&object.key).await?;
                    let relative = object.key.strip_prefix(&format!("{}/", path)).unwrap_or(&object.key);
                    let dest_path = dest_dir.join(relative);
                    if let Some(parent) = dest_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    tokio::fs::write(&dest_path, data.as_slice()).await?;
                }
            }
            Ok(true)
        } else {
            let src = Path::new(&self.local_dir).join(&path);
            if src.exists() {
                Self::copy_dir_recursive(&src, dest_dir).await?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    /// Check if artifacts exist for a deployment
    pub async fn has_artifacts(&self, deployment_id: &str) -> bool {
        if self.use_s3 {
            let bucket = self.bucket.as_ref().unwrap();
            let key = format!("artifacts/{}/", deployment_id);
            bucket.list(key, Some("/".to_string())).await
                .map(|r| !r.is_empty())
                .unwrap_or(false)
        } else {
            Path::new(&self.local_dir).join(deployment_id).exists()
        }
    }

    /// Get local path for serving (only works with local storage)
    pub fn local_artifacts_dir(&self, deployment_id: &str) -> String {
        format!("{}/{}", self.local_dir, deployment_id)
    }

    /// Recursive directory copy helper
    async fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<u64> {
        tokio::fs::create_dir_all(dst).await?;
        let mut total: u64 = 0;

        let mut entries = tokio::fs::read_dir(src).await?;
        while let Some(entry) = entries.next_entry().await? {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                total += Box::pin(Self::copy_dir_recursive(&src_path, &dst_path)).await?;
            } else {
                total += tokio::fs::copy(&src_path, &dst_path).await?;
            }
        }

        Ok(total)
    }
}
