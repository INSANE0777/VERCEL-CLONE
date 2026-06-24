use sqlx::postgres::{PgPool, PgPoolOptions};
use crate::models::{Project, Deployment, EnvVar, BuildCache, Domain, MiddlewareRuleDb};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .idle_timeout(Some(std::time::Duration::from_secs(600)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)))
            .connect(database_url)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    async fn run_migrations(&self) -> anyhow::Result<()> {
        let statements = [
            r#"CREATE TABLE IF NOT EXISTS projects (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                name TEXT NOT NULL,
                github_repo_full_name TEXT NOT NULL UNIQUE,
                github_repo_url TEXT NOT NULL,
                production_branch TEXT NOT NULL DEFAULT 'main',
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )"#,
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
            )"#,
            r#"CREATE TABLE IF NOT EXISTS env_vars (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                environment TEXT NOT NULL DEFAULT 'production',
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                UNIQUE(project_id, key, environment)
            )"#,
            r#"CREATE TABLE IF NOT EXISTS build_caches (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                cache_key TEXT NOT NULL,
                storage_path TEXT NOT NULL,
                size_bytes BIGINT NOT NULL DEFAULT 0,
                last_used TIMESTAMPTZ NOT NULL DEFAULT now(),
                UNIQUE(project_id, cache_key)
            )"#,
            r#"CREATE TABLE IF NOT EXISTS domains (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                domain TEXT NOT NULL UNIQUE,
                verified BOOLEAN NOT NULL DEFAULT false,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )"#,
            "CREATE INDEX IF NOT EXISTS idx_deployments_project ON deployments(project_id)",
            "CREATE INDEX IF NOT EXISTS idx_deployments_status ON deployments(status)",
            "CREATE INDEX IF NOT EXISTS idx_deployments_branch ON deployments(branch)",
            "CREATE INDEX IF NOT EXISTS idx_env_vars_project ON env_vars(project_id)",
            "CREATE INDEX IF NOT EXISTS idx_build_caches_project ON build_caches(project_id)",
            "CREATE INDEX IF NOT EXISTS idx_domains_project ON domains(project_id)",
            r#"CREATE TABLE IF NOT EXISTS middleware_rules (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                rule_type TEXT NOT NULL,
                pattern TEXT NOT NULL,
                target TEXT NOT NULL,
                status_code INTEGER,
                header_name TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )"#,
            "CREATE INDEX IF NOT EXISTS idx_middleware_project ON middleware_rules(project_id)",
            r#"CREATE TABLE IF NOT EXISTS analytics_events (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                project_id UUID REFERENCES projects(id) ON DELETE CASCADE,
                deployment_id UUID REFERENCES deployments(id) ON DELETE CASCADE,
                event_type TEXT NOT NULL,
                framework TEXT,
                duration_secs INTEGER,
                is_production BOOLEAN,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )"#,
            "CREATE INDEX IF NOT EXISTS idx_analytics_project ON analytics_events(project_id)",
            "CREATE INDEX IF NOT EXISTS idx_analytics_type ON analytics_events(event_type)",
            "CREATE INDEX IF NOT EXISTS idx_analytics_created ON analytics_events(created_at)",
        ];

        for stmt in &statements {
            sqlx::query(stmt).execute(&self.pool).await?;
        }

        tracing::info!("PostgreSQL migrations complete");
        Ok(())
    }

    // ── Projects ──────────────────────────────────────────

    pub async fn create_project(
        &self,
        name: &str,
        github_repo_full_name: &str,
        github_repo_url: &str,
        production_branch: &str,
    ) -> anyhow::Result<Project> {
        let project = sqlx::query_as::<_, Project>(
            r#"INSERT INTO projects (name, github_repo_full_name, github_repo_url, production_branch)
               VALUES ($1, $2, $3, $4)
               RETURNING *"#
        )
        .bind(name)
        .bind(github_repo_full_name)
        .bind(github_repo_url)
        .bind(production_branch)
        .fetch_one(&self.pool)
        .await?;
        Ok(project)
    }

    pub async fn get_project(&self, id: uuid::Uuid) -> anyhow::Result<Project> {
        let project = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(project)
    }

    pub async fn get_project_by_repo(&self, repo_full_name: &str) -> anyhow::Result<Project> {
        let project = sqlx::query_as::<_, Project>(
            "SELECT * FROM projects WHERE github_repo_full_name = $1",
        )
        .bind(repo_full_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(project)
    }

    pub async fn list_projects(&self) -> anyhow::Result<Vec<Project>> {
        let projects = sqlx::query_as::<_, Project>(
            "SELECT * FROM projects ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(projects)
    }

    pub async fn delete_project(&self, id: uuid::Uuid) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Deployments ───────────────────────────────────────

    pub async fn create_deployment(
        &self,
        project_id: uuid::Uuid,
        sha: &str,
        branch: &str,
        is_production: bool,
        url: &str,
    ) -> anyhow::Result<Deployment> {
        let deployment = sqlx::query_as::<_, Deployment>(
            r#"INSERT INTO deployments (project_id, sha, branch, status, url, is_production)
               VALUES ($1, $2, $3, 'queued', $4, $5)
               RETURNING *"#
        )
        .bind(project_id)
        .bind(sha)
        .bind(branch)
        .bind(url)
        .bind(is_production)
        .fetch_one(&self.pool)
        .await?;
        Ok(deployment)
    }

    pub async fn set_deployment_github_comment(
        &self,
        deployment_id: uuid::Uuid,
        comment_id: i64,
        pr_number: i32,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE deployments SET github_comment_id = $1, github_pr_number = $2 WHERE id = $3",
        )
        .bind(comment_id)
        .bind(pr_number)
        .bind(deployment_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_deployment_github_comment(
        &self,
        deployment_id: uuid::Uuid,
    ) -> anyhow::Result<Option<(i64, i32, uuid::Uuid, String, String)>> {
        let row: Option<(i64, i32, uuid::Uuid, String, String)> = sqlx::query_as(
            r#"SELECT d.github_comment_id, d.github_pr_number, d.project_id, p.github_repo_full_name, d.url
               FROM deployments d
               JOIN projects p ON d.project_id = p.id
               WHERE d.id = $1 AND d.github_comment_id IS NOT NULL"#
        )
        .bind(deployment_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get_deployment(&self, id: uuid::Uuid) -> anyhow::Result<Deployment> {
        let deployment = sqlx::query_as::<_, Deployment>(
            "SELECT * FROM deployments WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(deployment)
    }

    pub async fn get_latest_deployment(
        &self,
        project_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Deployment>> {
        let deployment = sqlx::query_as::<_, Deployment>(
            "SELECT * FROM deployments WHERE project_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(deployment)
    }

    pub async fn list_deployments(
        &self,
        project_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<Deployment>> {
        let deployments = sqlx::query_as::<_, Deployment>(
            "SELECT * FROM deployments WHERE project_id = $1 ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(deployments)
    }

    pub async fn update_deployment_status(
        &self,
        id: uuid::Uuid,
        status: &str,
        logs: Option<&str>,
        framework: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"UPDATE deployments 
               SET status = $1, build_logs = COALESCE($2, build_logs), framework = COALESCE($3, framework), updated_at = now()
               WHERE id = $4"#
        )
        .bind(status)
        .bind(logs)
        .bind(framework)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_active_build_count(&self) -> anyhow::Result<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM deployments WHERE status = 'building'",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count.0)
    }

    // ── Environment Variables ─────────────────────────────

    pub async fn set_env_var(
        &self,
        project_id: uuid::Uuid,
        key: &str,
        value: &str,
        environment: &str,
    ) -> anyhow::Result<EnvVar> {
        let env_var = sqlx::query_as::<_, EnvVar>(
            r#"INSERT INTO env_vars (project_id, key, value, environment)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (project_id, key, environment) 
               DO UPDATE SET value = EXCLUDED.value
               RETURNING *"#
        )
        .bind(project_id)
        .bind(key)
        .bind(value)
        .bind(environment)
        .fetch_one(&self.pool)
        .await?;
        Ok(env_var)
    }

    pub async fn get_env_vars(
        &self,
        project_id: uuid::Uuid,
        environment: &str,
    ) -> anyhow::Result<Vec<EnvVar>> {
        let vars = sqlx::query_as::<_, EnvVar>(
            "SELECT * FROM env_vars WHERE project_id = $1 AND environment = $2",
        )
        .bind(project_id)
        .bind(environment)
        .fetch_all(&self.pool)
        .await?;
        Ok(vars)
    }

    pub async fn delete_env_var(
        &self,
        project_id: uuid::Uuid,
        key: &str,
        environment: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "DELETE FROM env_vars WHERE project_id = $1 AND key = $2 AND environment = $3",
        )
        .bind(project_id)
        .bind(key)
        .bind(environment)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Build Cache ───────────────────────────────────────

    pub async fn get_build_cache(
        &self,
        project_id: uuid::Uuid,
        cache_key: &str,
    ) -> anyhow::Result<Option<BuildCache>> {
        let cache = sqlx::query_as::<_, BuildCache>(
            "SELECT * FROM build_caches WHERE project_id = $1 AND cache_key = $2",
        )
        .bind(project_id)
        .bind(cache_key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(cache)
    }

    pub async fn upsert_build_cache(
        &self,
        project_id: uuid::Uuid,
        cache_key: &str,
        storage_path: &str,
        size_bytes: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO build_caches (project_id, cache_key, storage_path, size_bytes, last_used)
               VALUES ($1, $2, $3, $4, now())
               ON CONFLICT (project_id, cache_key)
               DO UPDATE SET storage_path = EXCLUDED.storage_path, size_bytes = EXCLUDED.size_bytes, last_used = now()"#
        )
        .bind(project_id)
        .bind(cache_key)
        .bind(storage_path)
        .bind(size_bytes)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Domains ───────────────────────────────────────────

    pub async fn add_domain(
        &self,
        project_id: uuid::Uuid,
        domain: &str,
    ) -> anyhow::Result<Domain> {
        let d = sqlx::query_as::<_, Domain>(
            "INSERT INTO domains (project_id, domain) VALUES ($1, $2) RETURNING *",
        )
        .bind(project_id)
        .bind(domain)
        .fetch_one(&self.pool)
        .await?;
        Ok(d)
    }

    pub async fn get_domain(&self, domain: &str) -> anyhow::Result<Option<Domain>> {
        let d = sqlx::query_as::<_, Domain>(
            "SELECT * FROM domains WHERE domain = $1",
        )
        .bind(domain)
        .fetch_optional(&self.pool)
        .await?;
        Ok(d)
    }

    pub async fn list_domains(&self, project_id: uuid::Uuid) -> anyhow::Result<Vec<Domain>> {
        let domains = sqlx::query_as::<_, Domain>(
            "SELECT * FROM domains WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(domains)
    }

    pub async fn verify_domain(&self, domain: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE domains SET verified = true WHERE domain = $1")
            .bind(domain)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_domain(&self, project_id: uuid::Uuid, domain: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM domains WHERE project_id = $1 AND domain = $2")
            .bind(project_id)
            .bind(domain)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Middleware Rules ────────────────────────────────────

    pub async fn create_middleware_rule(
        &self,
        project_id: uuid::Uuid,
        rule_type: &str,
        pattern: &str,
        target: &str,
        status_code: Option<i32>,
        header_name: Option<&str>,
    ) -> anyhow::Result<MiddlewareRuleDb> {
        let rule = sqlx::query_as::<_, MiddlewareRuleDb>(
            r#"INSERT INTO middleware_rules (project_id, rule_type, pattern, target, status_code, header_name)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING *"#
        )
        .bind(project_id)
        .bind(rule_type)
        .bind(pattern)
        .bind(target)
        .bind(status_code)
        .bind(header_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(rule)
    }

    pub async fn list_middleware_rules(&self, project_id: uuid::Uuid) -> anyhow::Result<Vec<MiddlewareRuleDb>> {
        let rules = sqlx::query_as::<_, MiddlewareRuleDb>(
            "SELECT * FROM middleware_rules WHERE project_id = $1 ORDER BY created_at ASC"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rules)
    }

    pub async fn delete_middleware_rule(&self, project_id: uuid::Uuid, rule_id: uuid::Uuid) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM middleware_rules WHERE project_id = $1 AND id = $2")
            .bind(project_id)
            .bind(rule_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Analytics ───────────────────────────────────────────

    pub async fn record_analytics_event(
        &self,
        project_id: Option<uuid::Uuid>,
        deployment_id: Option<uuid::Uuid>,
        event_type: &str,
        framework: Option<&str>,
        duration_secs: Option<i32>,
        is_production: Option<bool>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO analytics_events (project_id, deployment_id, event_type, framework, duration_secs, is_production)
               VALUES ($1, $2, $3, $4, $5, $6)"#
        )
        .bind(project_id)
        .bind(deployment_id)
        .bind(event_type)
        .bind(framework)
        .bind(duration_secs)
        .bind(is_production)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn analytics_summary(&self) -> anyhow::Result<serde_json::Value> {
        let total_deploys: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics_events WHERE event_type = 'deployment_created'"
        ).fetch_one(&self.pool).await?;

        let total_builds: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics_events WHERE event_type = 'build_completed'"
        ).fetch_one(&self.pool).await?;

        let successful_builds: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics_events WHERE event_type = 'build_completed' AND duration_secs IS NOT NULL"
        ).fetch_one(&self.pool).await?;

        let failed_builds: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics_events WHERE event_type = 'build_failed'"
        ).fetch_one(&self.pool).await?;

        let avg_duration: (Option<f64>,) = sqlx::query_as(
            "SELECT AVG(duration_secs::float) FROM analytics_events WHERE event_type = 'build_completed' AND duration_secs IS NOT NULL"
        ).fetch_one(&self.pool).await?;

        let total_projects: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM projects"
        ).fetch_one(&self.pool).await?;

        let framework_counts: Vec<(String, i64)> = sqlx::query_as(
            "SELECT framework, COUNT(*) as cnt FROM analytics_events WHERE event_type = 'build_completed' AND framework IS NOT NULL GROUP BY framework ORDER BY cnt DESC"
        ).fetch_all(&self.pool).await?;

        let last_7_days: Vec<(String, i64)> = sqlx::query_as(
            r#"SELECT to_char(date_trunc('day', created_at), 'YYYY-MM-DD') as day, COUNT(*) as cnt
               FROM analytics_events
               WHERE event_type = 'deployment_created' AND created_at > now() - interval '7 days'
               GROUP BY day ORDER BY day"#
        ).fetch_all(&self.pool).await?;

        let success_rate = if total_builds.0 > 0 {
            (successful_builds.0 as f64 / total_builds.0 as f64) * 100.0
        } else {
            0.0
        };

        Ok(serde_json::json!({
            "total_projects": total_projects.0,
            "total_deployments": total_deploys.0,
            "total_builds": total_builds.0,
            "successful_builds": successful_builds.0,
            "failed_builds": failed_builds.0,
            "success_rate": (success_rate * 10.0).round() / 10.0,
            "avg_build_duration_secs": avg_duration.0.map(|d| (d * 10.0).round() / 10.0).unwrap_or(0.0),
            "frameworks": framework_counts.into_iter().map(|(f, c)| serde_json::json!({"framework": f, "count": c})).collect::<Vec<_>>(),
            "deploys_last_7_days": last_7_days.into_iter().map(|(d, c)| serde_json::json!({"date": d, "count": c})).collect::<Vec<_>>(),
        }))
    }

    pub async fn project_analytics(&self, project_id: uuid::Uuid) -> anyhow::Result<serde_json::Value> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM deployments WHERE project_id = $1"
        ).bind(project_id).fetch_one(&self.pool).await?;

        let ready: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM deployments WHERE project_id = $1 AND status = 'ready'"
        ).bind(project_id).fetch_one(&self.pool).await?;

        let errors: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM deployments WHERE project_id = $1 AND status = 'error'"
        ).bind(project_id).fetch_one(&self.pool).await?;

        let avg_dur: (Option<f64>,) = sqlx::query_as(
            r#"SELECT AVG(duration_secs::float) FROM analytics_events
               WHERE project_id = $1 AND event_type = 'build_completed' AND duration_secs IS NOT NULL"#
        ).bind(project_id).fetch_one(&self.pool).await?;

        let recent: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
            r#"SELECT d.id::text, d.status, d.branch, d.framework
               FROM deployments d WHERE d.project_id = $1
               ORDER BY d.created_at DESC LIMIT 10"#
        ).bind(project_id).fetch_all(&self.pool).await?;

        Ok(serde_json::json!({
            "total_deployments": total.0,
            "ready": ready.0,
            "errors": errors.0,
            "avg_build_duration_secs": avg_dur.0.map(|d| (d * 10.0).round() / 10.0).unwrap_or(0.0),
            "recent_deployments": recent.into_iter().map(|(id, status, branch, fw)| serde_json::json!({
                "id": id, "status": status, "branch": branch, "framework": fw
            })).collect::<Vec<_>>(),
        }))
    }
}
