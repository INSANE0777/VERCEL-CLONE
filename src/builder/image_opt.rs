use image::ImageFormat;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Optimize images in a build output directory.
/// Converts PNG/JPEG/GIF to WebP (quality 80) and re-encodes originals with
/// better compression. Originals are kept; WebP versions sit alongside them.
///
/// Returns (files_optimized, bytes_saved_estimate).
pub async fn optimize_images(artifact_dir: &Path) -> anyhow::Result<(usize, u64)> {
    let mut optimized = 0;
    let mut saved_bytes: u64 = 0;

    let image_files: Vec<PathBuf> = WalkDir::new(artifact_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            matches!(
                e.path().extension().and_then(|ext| ext.to_str()),
                Some("png") | Some("jpg") | Some("jpeg") | Some("gif")
            )
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    for img_path in image_files {
        let original_size = match tokio::fs::metadata(&img_path).await {
            Ok(m) => m.len(),
            Err(_) => continue,
        };

        let ext = img_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        let webp_path = img_path.with_extension("webp");

        // Skip if WebP already exists (idempotent on re-runs)
        if webp_path.exists() {
            continue;
        }

        // Re-encode original with optimized compression
        let re_encode_path = img_path.clone();
        let re_encode_ext = ext.clone();
        let re_encode_result = tokio::task::spawn_blocking(move || {
            let img = image::open(&re_encode_path)?;
            let format = match re_encode_ext.as_str() {
                "jpg" | "jpeg" => ImageFormat::Jpeg,
                "png" => ImageFormat::Png,
                "gif" => ImageFormat::Gif,
                _ => return Ok::<(), anyhow::Error>(()),
            };
            let mut buf = std::io::BufWriter::new(std::fs::File::create(&re_encode_path)?);
            img.write_to(&mut buf, format)?;
            Ok(())
        })
        .await;

        // Generate WebP version
        let webp_src = img_path.clone();
        let webp_dst = webp_path.clone();
        let webp_result = tokio::task::spawn_blocking(move || {
            let img = image::open(&webp_src)?;
            let mut buf = std::io::BufWriter::new(std::fs::File::create(&webp_dst)?);
            img.write_to(&mut buf, ImageFormat::WebP)?;
            Ok::<(), anyhow::Error>(())
        })
        .await;

        if re_encode_result.is_ok() && webp_result.is_ok() {
            let new_size = tokio::fs::metadata(&img_path).await.map(|m| m.len()).unwrap_or(0);
            let webp_size = tokio::fs::metadata(&webp_path).await.map(|m| m.len()).unwrap_or(0);

            if webp_size < original_size {
                saved_bytes += original_size - webp_size;
            }
            if new_size < original_size {
                saved_bytes += original_size - new_size;
            }

            optimized += 1;
        }
    }

    tracing::info!(
        "Image optimization: {} files optimized, ~{} bytes saved",
        optimized,
        saved_bytes
    );

    Ok((optimized, saved_bytes))
}
