use std::path::Path;

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

    // Generic: api/ or server/api/ or functions/ in build output
    for api_dir in &["api", "server/api", "functions"] {
        let path = build_path.join(api_dir);
        if path.exists() {
            collect_route_files(&path, &mut routes);
        }
    }

    routes
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
