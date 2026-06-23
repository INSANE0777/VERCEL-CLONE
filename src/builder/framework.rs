use std::path::Path;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Framework {
    pub name: String,
    pub install_command: String,
    pub build_command: String,
    pub output_dir: String,
}

pub async fn detect_framework(project_dir: &Path) -> Result<Framework> {
    let package_json_path = project_dir.join("package.json");

    if !package_json_path.exists() {
        return Ok(Framework {
            name: "static".into(),
            install_command: "echo 'no install needed'".into(),
            build_command: "echo 'no build needed'".into(),
            output_dir: ".".into(),
        });
    }

    let content = tokio::fs::read_to_string(&package_json_path).await?;
    let package_json: serde_json::Value = serde_json::from_str(&content)?;

    let deps = package_json.get("dependencies")
        .and_then(|d| d.as_object()).cloned().unwrap_or_default();
    let dev_deps = package_json.get("devDependencies")
        .and_then(|d| d.as_object()).cloned().unwrap_or_default();

    let all: std::collections::HashMap<String, _> = deps.iter()
        .chain(dev_deps.iter()).map(|(k, v)| (k.clone(), v)).collect();

    let install_cmd = if project_dir.join("pnpm-lock.yaml").exists() {
        "pnpm install --frozen-lockfile"
    } else if project_dir.join("yarn.lock").exists() {
        "yarn install --frozen-lockfile"
    } else if project_dir.join("bun.lockb").exists() {
        "bun install"
    } else {
        "npm ci"
    };

    let mut result = None;

    if all.contains_key("next") {
        result = Some(Framework {
            name: "nextjs".into(), install_command: install_cmd.into(),
            build_command: "npx next build".into(), output_dir: ".next".into(),
        });
    } else if all.contains_key("@remix-run/react") || all.contains_key("@remix-run/dev") {
        result = Some(Framework {
            name: "remix".into(), install_command: install_cmd.into(),
            build_command: "npx remix build".into(), output_dir: "build/client".into(),
        });
    } else if all.contains_key("astro") {
        result = Some(Framework {
            name: "astro".into(), install_command: install_cmd.into(),
            build_command: "npx astro build".into(), output_dir: "dist".into(),
        });
    } else if all.contains_key("nuxt") {
        result = Some(Framework {
            name: "nuxt".into(), install_command: install_cmd.into(),
            build_command: "npx nuxt build".into(), output_dir: ".output/public".into(),
        });
    } else if all.contains_key("@sveltejs/kit") {
        result = Some(Framework {
            name: "sveltekit".into(), install_command: install_cmd.into(),
            build_command: "npx svelte-kit build".into(), output_dir: "build".into(),
        });
    } else if all.contains_key("vite") {
        result = Some(Framework {
            name: "vite".into(), install_command: install_cmd.into(),
            build_command: "npx vite build".into(), output_dir: "dist".into(),
        });
    } else if all.contains_key("react-scripts") {
        result = Some(Framework {
            name: "create-react-app".into(), install_command: install_cmd.into(),
            build_command: "npx react-scripts build".into(), output_dir: "build".into(),
        });
    } else if all.contains_key("gatsby") {
        result = Some(Framework {
            name: "gatsby".into(), install_command: install_cmd.into(),
            build_command: "npx gatsby build".into(), output_dir: "public".into(),
        });
    }

    if let Some(fw) = result {
        return Ok(fw);
    }

    let scripts = package_json.get("scripts")
        .and_then(|s| s.as_object()).cloned().unwrap_or_default();

    if scripts.contains_key("build") {
        let pm = install_cmd.split_whitespace().next().unwrap_or("npm");
        return Ok(Framework {
            name: "generic-node".into(), install_command: install_cmd.into(),
            build_command: format!("{} run build", pm), output_dir: "dist".into(),
        });
    }

    Ok(Framework {
        name: "static".into(), install_command: "echo 'no install needed'".into(),
        build_command: "echo 'no build needed'".into(), output_dir: ".".into(),
    })
}
