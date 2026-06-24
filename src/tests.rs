// ═══════════════════════════════════════════════════════════════
// Tests for vercel-clone — covers bugs we actually hit + core logic
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod framework_tests {
    use crate::builder::framework::detect_framework;

    #[tokio::test]
    async fn detects_vite() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"),
            r#"{"dependencies": {}, "devDependencies": {"vite": "^5.0.0"}}"#).unwrap();
        let fw = detect_framework(dir.path()).await.unwrap();
        assert_eq!(fw.name, "vite");
        assert_eq!(fw.build_command, "npx vite build");
        assert_eq!(fw.output_dir, "dist");
    }

    #[tokio::test]
    async fn detects_nextjs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"),
            r#"{"dependencies": {"next": "^14.0.0"}, "devDependencies": {}}"#).unwrap();
        let fw = detect_framework(dir.path()).await.unwrap();
        assert_eq!(fw.name, "nextjs");
        assert_eq!(fw.output_dir, ".next");
    }

    #[tokio::test]
    async fn detects_static_when_no_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let fw = detect_framework(dir.path()).await.unwrap();
        assert_eq!(fw.name, "static");
        assert_eq!(fw.build_command, "echo 'no build needed'");
    }

    #[tokio::test]
    async fn detects_astro() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"),
            r#"{"dependencies": {"astro": "^4.0.0"}, "devDependencies": {}}"#).unwrap();
        let fw = detect_framework(dir.path()).await.unwrap();
        assert_eq!(fw.name, "astro");
        assert_eq!(fw.output_dir, "dist");
    }

    #[tokio::test]
    async fn detects_generic_node_with_build_script() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"),
            r#"{"dependencies": {"some-random-pkg": "^1.0.0"}, "devDependencies": {}, "scripts": {"build": "webpack"}}"#).unwrap();
        let fw = detect_framework(dir.path()).await.unwrap();
        assert_eq!(fw.name, "generic-node");
        assert!(fw.build_command.contains("run build"));
    }

    #[tokio::test]
    async fn uses_pnpm_when_lockfile_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"),
            r#"{"dependencies": {"vite": "^5.0.0"}, "devDependencies": {}}"#).unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        let fw = detect_framework(dir.path()).await.unwrap();
        assert!(fw.install_command.starts_with("pnpm install"));
    }

    #[tokio::test]
    async fn uses_yarn_when_lockfile_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"),
            r#"{"dependencies": {"vite": "^5.0.0"}, "devDependencies": {}}"#).unwrap();
        std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
        let fw = detect_framework(dir.path()).await.unwrap();
        assert!(fw.install_command.starts_with("yarn install"));
    }
}

#[cfg(test)]
mod functions_tests {
    use crate::builder::functions::detect_api_routes;

    #[test]
    fn detects_generic_api_routes() {
        let dir = tempfile::tempdir().unwrap();
        let api = dir.path().join("api");
        std::fs::create_dir_all(&api).unwrap();
        std::fs::write(api.join("hello.js"), "module.exports = {};").unwrap();
        std::fs::write(api.join("users.js"), "module.exports = {};").unwrap();
        let routes = detect_api_routes(dir.path(), "generic");
        assert!(routes.contains(&"/hello".to_string()));
        assert!(routes.contains(&"/users".to_string()));
    }

    #[test]
    fn returns_empty_for_no_api_dir() {
        let dir = tempfile::tempdir().unwrap();
        let routes = detect_api_routes(dir.path(), "vite");
        assert!(routes.is_empty());
    }

    #[test]
    fn detects_functions_dir() {
        let dir = tempfile::tempdir().unwrap();
        let fns = dir.path().join("functions");
        std::fs::create_dir_all(&fns).unwrap();
        std::fs::write(fns.join("health.js"), "export default () => {};").unwrap();
        let routes = detect_api_routes(dir.path(), "generic");
        assert!(routes.contains(&"/health".to_string()));
    }
}

#[cfg(test)]
mod url_normalization_tests {
    // Tests the bug where https://github.com/ was doubled.
    // This is the exact normalization logic from create_project handler.

    fn normalize_repo_input(input: &str) -> String {
        input
            .trim_start_matches("https://github.com/")
            .trim_start_matches("http://github.com/")
            .trim_start_matches("github.com/")
            .trim_end_matches('/')
            .trim()
            .to_string()
    }

    #[test]
    fn strips_full_https_url() {
        assert_eq!(
            normalize_repo_input("https://github.com/INSANE0777/AI-MAYHEM"),
            "INSANE0777/AI-MAYHEM"
        );
    }

    #[test]
    fn strips_full_http_url() {
        assert_eq!(
            normalize_repo_input("http://github.com/INSANE0777/AI-MAYHEM"),
            "INSANE0777/AI-MAYHEM"
        );
    }

    #[test]
    fn strips_github_com_prefix() {
        assert_eq!(
            normalize_repo_input("github.com/INSANE0777/AI-MAYHEM"),
            "INSANE0777/AI-MAYHEM"
        );
    }

    #[test]
    fn keeps_owner_repo_format() {
        assert_eq!(
            normalize_repo_input("INSANE0777/AI-MAYHEM"),
            "INSANE0777/AI-MAYHEM"
        );
    }

    #[test]
    fn strips_trailing_slash() {
        assert_eq!(
            normalize_repo_input("https://github.com/INSANE0777/AI-MAYHEM/"),
            "INSANE0777/AI-MAYHEM"
        );
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(
            normalize_repo_input("  INSANE0777/AI-MAYHEM  "),
            "INSANE0777/AI-MAYHEM"
        );
    }

    #[test]
    fn reconstructed_url_is_not_doubled() {
        let input = "https://github.com/INSANE0777/AI-MAYHEM";
        let normalized = normalize_repo_input(input);
        let url = format!("https://github.com/{}", normalized);
        assert_eq!(url, "https://github.com/INSANE0777/AI-MAYHEM");
        assert!(!url.contains("github.com/https://"));
    }
}

#[cfg(test)]
mod caddy_config_tests {
    use crate::edge::EdgeRouter;

    #[tokio::test]
    async fn generates_localhost_config_with_port_80() {
        let dir = tempfile::tempdir().unwrap();
        let router = EdgeRouter::new(
            dir.path().to_str().unwrap(),
            "localhost",
        );

        router
            .add_deployment(
                "test-deploy-id",
                "myapp.localhost",
                "/artifacts/test-deploy-id/dist",
                "vite",
                &[],
            )
            .await
            .unwrap();

        let config_path = dir.path().join("myapp_localhost.caddy");
        assert!(config_path.exists(), "Caddy config file should exist");

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("myapp.localhost:80"), "Must listen on :80");
        assert!(content.contains("/artifacts/test-deploy-id/dist"));
        assert!(content.contains("file_server"));
        assert!(content.contains("try_files"));
    }

    #[tokio::test]
    async fn removes_deployment_config() {
        let dir = tempfile::tempdir().unwrap();
        let router = EdgeRouter::new(dir.path().to_str().unwrap(), "localhost");

        router
            .add_deployment("id1", "app.localhost", "/artifacts/id1", "vite", &[])
            .await
            .unwrap();

        let config_path = dir.path().join("app_localhost.caddy");
        assert!(config_path.exists());

        router.remove_deployment("app.localhost").await.unwrap();
        assert!(!config_path.exists(), "Config should be removed");
    }

    #[tokio::test]
    async fn config_does_not_have_tls_block_on_localhost() {
        let dir = tempfile::tempdir().unwrap();
        let router = EdgeRouter::new(dir.path().to_str().unwrap(), "localhost");

        router
            .add_deployment("id1", "app.localhost", "/artifacts/id1", "vite", &[])
            .await
            .unwrap();

        let content = std::fs::read_to_string(
            dir.path().join("app_localhost.caddy"),
        )
        .unwrap();

        assert!(!content.contains("tls {"), "TLS block must not appear in localhost config");
    }
}

#[cfg(test)]
mod config_tests {
    use crate::config::AppConfig;

    #[test]
    fn loads_defaults_without_env() {
        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.port, 3000);
        assert_eq!(config.base_domain, "localhost");
        assert_eq!(config.max_concurrent_builds, 4);
        assert_eq!(config.build_timeout_secs, 600);
        assert_eq!(config.nats_stream_name, "BUILDS");
    }

    #[test]
    fn build_memory_limit_is_parseable() {
        let config = AppConfig::from_env().unwrap();
        assert!(config.build_memory_limit > 0);
    }
}

#[cfg(test)]
mod migration_tests {
    // Tests that migration SQL statements are individually executable.
    // This is the bug we hit: PostgreSQL rejects multiple statements in
    // a single prepared statement.

    fn get_migration_statements() -> Vec<String> {
        // Mirror the exact statements from db/mod.rs run_migrations
        vec![
            r#"CREATE TABLE IF NOT EXISTS projects (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                name TEXT NOT NULL,
                github_repo_full_name TEXT NOT NULL UNIQUE,
                github_repo_url TEXT NOT NULL,
                production_branch TEXT NOT NULL DEFAULT 'main',
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )"#.to_string(),
            r#"CREATE TABLE IF NOT EXISTS deployments (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                sha TEXT NOT NULL,
                branch TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'queued',
                build_logs TEXT,
                framework TEXT,
                url TEXT,
                is_production BOOLEAN NOT NULL DEFAULT false,
                github_comment_id BIGINT,
                github_pr_number INTEGER,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )"#.to_string(),
            r#"CREATE TABLE IF NOT EXISTS env_vars (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                environment TEXT NOT NULL DEFAULT 'production',
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                UNIQUE(project_id, key, environment)
            )"#.to_string(),
            r#"CREATE TABLE IF NOT EXISTS build_caches (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                cache_key TEXT NOT NULL,
                storage_path TEXT NOT NULL,
                size_bytes BIGINT NOT NULL DEFAULT 0,
                last_used TIMESTAMPTZ NOT NULL DEFAULT now(),
                UNIQUE(project_id, cache_key)
            )"#.to_string(),
            r#"CREATE TABLE IF NOT EXISTS domains (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                domain TEXT NOT NULL UNIQUE,
                verified BOOLEAN NOT NULL DEFAULT false,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )"#.to_string(),
            "CREATE INDEX IF NOT EXISTS idx_deployments_project ON deployments(project_id)".to_string(),
            "CREATE INDEX IF NOT EXISTS idx_deployments_status ON deployments(status)".to_string(),
            "CREATE INDEX IF NOT EXISTS idx_deployments_branch ON deployments(branch)".to_string(),
            r#"CREATE TABLE IF NOT EXISTS middleware_rules (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                rule_type TEXT NOT NULL,
                pattern TEXT NOT NULL,
                target TEXT NOT NULL,
                status_code INTEGER,
                header_name TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )"#.to_string(),
            "CREATE INDEX IF NOT EXISTS idx_middleware_project ON middleware_rules(project_id)".to_string(),
            r#"CREATE TABLE IF NOT EXISTS analytics_events (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID REFERENCES projects(id) ON DELETE CASCADE,
                deployment_id UUID REFERENCES deployments(id) ON DELETE CASCADE,
                event_type TEXT NOT NULL,
                framework TEXT,
                duration_secs INTEGER,
                is_production BOOLEAN,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )"#.to_string(),
            "CREATE INDEX IF NOT EXISTS idx_analytics_project ON analytics_events(project_id)".to_string(),
            "CREATE INDEX IF NOT EXISTS idx_analytics_type ON analytics_events(event_type)".to_string(),
            "CREATE INDEX IF NOT EXISTS idx_analytics_created ON analytics_events(created_at)".to_string(),
        ]
    }

    fn count_top_level_semicolons(sql: &str) -> usize {
        let mut count = 0;
        let mut in_string = false;
        let mut prev_char = ' ';

        for ch in sql.chars() {
            if ch == '\'' && prev_char != '\\' {
                in_string = !in_string;
            }
            if ch == ';' && !in_string {
                count += 1;
            }
            prev_char = ch;
        }
        count
    }

    #[test]
    fn migration_statements_are_single_commands() {
        let statements = get_migration_statements();
        for (i, stmt) in statements.iter().enumerate() {
            let trimmed = stmt.trim();
            assert!(!trimmed.is_empty(), "Statement {} is empty", i);
            let semicolons = count_top_level_semicolons(trimmed);
            assert_eq!(
                semicolons, 0,
                "Statement {} contains multiple SQL commands ({} semicolons) — \
                 PostgreSQL prepared statements only allow one command per query",
                i, semicolons,
            );
        }
    }

    #[test]
    fn migration_creates_projects_table() {
        let statements = get_migration_statements();
        let combined = statements.join(" ");
        assert!(combined.contains("CREATE TABLE IF NOT EXISTS projects"));
    }

    #[test]
    fn migration_creates_deployments_table() {
        let statements = get_migration_statements();
        let combined = statements.join(" ");
        assert!(combined.contains("CREATE TABLE IF NOT EXISTS deployments"));
    }

    #[test]
    fn migration_creates_analytics_table() {
        let statements = get_migration_statements();
        let combined = statements.join(" ");
        assert!(combined.contains("CREATE TABLE IF NOT EXISTS analytics_events"));
    }
}
