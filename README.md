# 🚀 vercel-clone v0.2.0 — Production-Grade Deployment Platform in Rust

A self-hosted, Vercel-inspired deployment pipeline built from scratch in Rust. Push code to GitHub → NATS JetStream queues the build → multiple Rust workers clone/build/deploy in isolated Docker containers → Caddy serves with auto-HTTPS. PostgreSQL for state, S3/MinIO for artifacts, WebSocket streams for real-time logs.

## Architecture v0.2.0 (Production-Grade)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         COMPLETE STACK                                       │
│                                                                              │
│  ┌──────────────┐   ┌──────────────┐   ┌─────────────────────────────────┐ │
│  │  GitHub      │──▶│  Axum API    │──▶│  NATS JetStream (Work Queue)   │ │
│  │  Webhook     │   │  :3000       │   │  BUILDS stream                 │ │
│  └──────────────┘   │ Stateless    │   │  Exactly-once delivery         │ │
│                     │ PostgreSQL   │   └─────────────┬─────────────────┘ │
│                     │ stateless    │                   │                  │
│                     └──────────────┘                   │                  │
│                                                        ▼                  │
│                     ┌─────────────────────────┐ ┌────────────────────────┐│
│                     │  PostgreSQL (shared DB) │ │ Build Worker Pool      ││
│                     │  Projects/Deployments/    │ │ (4+ concurrent workers)│
│                     │  Env Vars/Domains       │ │ Docker build isolation ││
│                     └─────────────────────────┘ │ Cache restore/save     ││
│                                                 │ S3 artifact upload     ││
│                                                 │ Caddy config generation  ││
│                                                 └────────────────────────┘│
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │  Caddy :80/:443 (Auto-HTTPS reverse proxy)                           │  │
│  │  ├─ /api/*        → proxy to api:3000                              │  │
│  │  ├─ /webhooks/*   → proxy to api:3000                              │  │
│  │  ├─ *.localhost   → static files from /artifacts (or S3 presigned)   │  │
│  │  └─ Auto-generated .caddy configs per deployment                      │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │  Object Storage: MinIO/S3 (S3-compatible API)                         │  │
│  │  ├─ artifacts/{deployment_id}/   ← build output                       │  │
│  │  └─ cache/{project_id}/{key}/    ← node_modules cache                │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

## What's New in v0.2.0

| Feature | v0.1 (MVP) | v0.2 (Production) |
|---------|-----------|------------------|
| **Database** | SQLite (single writer, racy) | PostgreSQL (20 conn pool, real ACID) |
| **Job Queue** | SQLite polling every 2s | NATS JetStream (push, <1ms, exactly-once) |
| **Artifact Storage** | Local filesystem only | S3/MinIO (distributed, persistent) |
| **Build Workers** | 1 single-threaded | 4+ concurrent, auto-scaling |
| **Serving** | Nginx manual config | Caddy (auto-HTTPS, gzip, security headers) |
| **Build Cache** | None — full install every time | node_modules cached by lockfile hash |
| **Env Variables** | Not supported | Per-project, per-environment (prod/preview) |
| **Custom Domains** | Not supported | Full domain management with verification |
| **Log Streaming** | Poll REST API | WebSocket real-time status updates |
| **Retry Logic** | None | 3-attempt retry with dead-letter queue |
| **Multi-tenancy** | Single machine | Horizontal worker scaling via NATS |

## Tech Stack

| Layer | Technology | Role |
|-------|-----------|------|
| **Web Server** | Axum + Tower | REST API + WebSocket log streaming |
| **Database** | PostgreSQL + SQLx | Shared state across all workers |
| **Job Queue** | NATS JetStream | Distributed, exactly-once build queue |
| **Build Isolation** | Docker + Bollard | Containerized builds with memory limits |
| **Artifact Store** | S3-compatible (MinIO) | Distributed, persistent build output |
| **Edge Router** | Caddy | Auto-HTTPS, gzip, SPA fallback, security headers |
| **Object Storage** | MinIO | S3-compatible, self-hosted |

## Quick Start

### Prerequisites
- [Docker & Docker Compose](https://docs.docker.com/get-docker/)
- Git

### 1. Clone & Configure

```bash
cd vercel-clone
cp .env.example .env
# Edit .env — at minimum set GITHUB_WEBHOOK_SECRET
```

### 2. Launch Full Stack

```bash
docker-compose up --build
```

This starts 6 services:
- `postgres` — PostgreSQL 16
- `nats` — NATS with JetStream
- `minio` — S3-compatible object storage
- `minio-init` — Creates the artifacts bucket
- `caddy` — Reverse proxy + auto-HTTPS
- `api` — The Rust application (API + build workers)

### 3. Create a Project

```bash
curl -X POST http://localhost:3000/api/projects \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-app",
    "github_repo_full_name": "your-username/your-repo",
    "production_branch": "main"
  }'
```

### 4. Set Environment Variables (optional)

```bash
# Production env vars
curl -X POST http://localhost:3000/api/projects/{id}/env \
  -H "Content-Type: application/json" \
  -d '{"key": "API_URL", "value": "https://api.example.com", "environment": "production"}'

# Preview env vars (for PR deployments)
curl -X POST http://localhost:3000/api/projects/{id}/env \
  -H "Content-Type: application/json" \
  -d '{"key": "API_URL", "value": "https://staging.example.com", "environment": "preview"}'
```

### 5. Trigger a Deploy

```bash
curl -X POST http://localhost:3000/api/projects/{id}/deploy \
  -H "Content-Type: application/json" \
  -d '{"branch": "main", "sha": "HEAD"}'
```

Or set up a GitHub webhook:
- **URL:** `http://your-server:3000/webhooks/github`
- **Content type:** `application/json`
- **Secret:** `GITHUB_WEBHOOK_SECRET` from `.env`
- **Events:** `push`

### 6. Watch Real-Time Logs via WebSocket

```bash
# Connect to WebSocket for live build output
wscat -c ws://localhost:3000/api/deployments/{deployment_id}/status/stream
```

Or use the REST endpoint:
```bash
curl http://localhost:3000/api/deployments/{deployment_id}/logs
```

### 7. View Deployed Site

Your deployment is served at the URL returned in the deploy response:
- Production: `http://my-app.localhost`
- Preview: `http://my-app-abc12345.localhost`

(Caddy handles routing automatically. For production domains, Caddy auto-provisions Let's Encrypt certificates.)

## API Reference

### Projects
```bash
GET    /api/projects              # List all projects
POST   /api/projects              # Create project
GET    /api/projects/:id          # Get project details
GET    /api/projects/:id/deployments  # List deployments
POST   /api/projects/:id/deploy       # Trigger manual deploy
GET    /api/projects/:id/env          # List env vars
POST   /api/projects/:id/env          # Set env var
GET    /api/projects/:id/domains      # List custom domains
POST   /api/projects/:id/domains      # Add custom domain
```

### Deployments
```bash
GET    /api/deployments/:id            # Get deployment status
GET    /api/deployments/:id/logs       # Get build logs
GET    /api/deployments/:id/status/stream  # WebSocket real-time status
```

### Platform
```bash
GET    /api/health    # Health check + queue depth + active builds
```

### Webhooks
```bash
POST   /webhooks/github    # GitHub push webhook (HMAC-verified)
```

## Project Structure

```
vercel-clone/
├── Cargo.toml                     # Rust dependencies
├── Dockerfile                     # Multi-stage build (Rust binary)
├── Dockerfile.build-runner        # Node.js build environment
├── docker-compose.yml             # Full stack orchestration
├── .env.example                   # Configuration template
├── .gitignore
├── caddy/
│   ├── Caddyfile                  # Main Caddy config (auto-HTTPS)
│   └── configs/                 # Auto-generated per-deployment configs
├── src/
│   ├── main.rs                    # Entry point — API + worker spawn
│   ├── config.rs                  # 12-typed env configuration
│   ├── models/
│   │   └── mod.rs                 # Project, Deployment, EnvVar, Domain, BuildJob
│   ├── db/
│   │   └── mod.rs                 # PostgreSQL layer (CRUD + migrations)
│   ├── queue/
│   │   └── mod.rs                 # NATS JetStream producer/consumer
│   ├── storage/
│   │   └── mod.rs                 # S3/MinIO artifact + cache storage
│   ├── edge/
│   │   └── mod.rs                 # Caddy config generation
│   ├── builder/
│   │   ├── mod.rs                 # Build worker (Docker execution, cache, S3 upload)
│   │   └── framework.rs           # Auto-detect Next.js, Vite, Remix, Astro, etc.
│   └── api/
│       ├── mod.rs                 # Axum router (WebSocket + REST)
│       └── handlers.rs            # All route handlers + GitHub webhook verification
├── nginx/                         # (legacy, replaced by Caddy)
└── data/                          # (gitignored — PostgreSQL + MinIO volumes)
```

## How a Build Works (End-to-End)

```
1. GitHub push → POST /webhooks/github
        │
        ▼
2. HMAC signature verified → parse payload
        │
        ▼
3. Look up project by repo name in PostgreSQL
        │
        ▼
4. Create deployment row (status: "queued")
        │
        ▼
5. Fetch env vars for this project + environment (prod vs preview)
        │
        ▼
6. Publish BuildJob to NATS JetStream "builds.new" subject
        │
        ▼
7. One of N build workers picks it up instantly (<1ms)
        │
        ▼
8. Worker marks deployment "building" in PostgreSQL
        │
        ▼
9. git clone --depth 1 <repo>
        │
        ▼
10. Detect framework from package.json (Next.js? Vite? Remix?)
        │
        ▼
11. Check build cache — SHA256(package-lock.json) as cache key
        │      ├─ Cache HIT → restore node_modules from S3 (skip npm install)
        │      └─ Cache MISS → npm ci inside Docker container
        │
        ▼
12. Run build inside Docker (memory limit, no network option)
        │      Stream stdout/stderr → logs
        │
        ▼
13. Upload build output to S3 /artifacts/{deployment_id}/
        │
        ▼
14. Save node_modules to cache (S3 /cache/{project_id}/{hash}/)
        │
        ▼
15. Generate Caddy .caddy config for deployment URL
        │
        ▼
16. Mark deployment "ready" in PostgreSQL
        │
        ▼
17. Publish status update to NATS "builds.status.{id}" (WebSocket picks up)
        │
        ▼
18. User visits URL → Caddy → static files → SPA fallback to index.html
```

## Supported Frameworks (Auto-Detected)

| Framework | Detection | Build Command | Output |
|-----------|-----------|---------------|--------|
| **Next.js** | `next` in deps | `npx next build` | `.next/` |
| **Remix** | `@remix-run/react` | `npx remix build` | `build/client/` |
| **Astro** | `astro` in deps | `npx astro build` | `dist/` |
| **Nuxt** | `nuxt` in deps | `npx nuxt build` | `.output/public/` |
| **SvelteKit** | `@sveltejs/kit` | `npx svelte-kit build` | `build/` |
| **Vite** | `vite` in deps | `npx vite build` | `dist/` |
| **Create React App** | `react-scripts` | `npx react-scripts build` | `build/` |
| **Gatsby** | `gatsby` in deps | `npx gatsby build` | `public/` |
| **Generic Node** | `build` script in package.json | `npm run build` | `dist/` |
| **Static** | `index.html` present | none | `.` |

Package manager auto-detected by lockfile: `pnpm-lock.yaml` → pnpm, `yarn.lock` → yarn, `bun.lockb` → bun, else `npm ci`.

## Scaling

### Vertical (Single Machine)
Set `MAX_CONCURRENT_BUILDS=8` — each worker runs builds sequentially but multiple workers run in parallel.

### Horizontal (Multiple Machines)
Add more `api` service replicas — all share PostgreSQL + NATS + S3:
```yaml
# docker-compose.yml
api:
  deploy:
    replicas: 4
```
Each replica runs both the REST API and build workers. The NATS consumer group ensures each build job is consumed by exactly one worker.

## Roadmap

### ✅ v0.2.0
- [x] PostgreSQL database with connection pooling
- [x] NATS JetStream distributed job queue
- [x] S3/MinIO artifact and cache storage
- [x] Multi-worker concurrent build system
- [x] Build caching (node_modules by lockfile hash)
- [x] Per-project, per-environment variables
- [x] Custom domain support
- [x] WebSocket real-time log streaming
- [x] Caddy auto-HTTPS edge router
- [x] 3-attempt retry with dead-letter queue
- [x] 10 framework auto-detection
- [x] Mandatory HMAC webhook verification
- [x] Build timeout enforcement
- [x] GitHub token support for private repos
- [x] DELETE routes (projects, env vars, domains)
- [x] Multi-lockfile cache key detection

### ✅ v0.3.0 (Current)
- [x] Firecracker microVM build isolation (hardware-enforced, like Vercel/Fly.io)
- [x] Warm pool of pre-booted build environments (<1s cold start)
- [x] Image optimization pipeline (PNG/JPEG/GIF → WebP + re-compress)
- [x] Serverless function execution (API route detection + Docker runtime)
- [x] Edge middleware (redirects, rewrites, custom headers → Caddy directives)

### 🔜 v0.3.1
- [x] GitHub PR comment bot (posts deployment URL on each PR)
- [ ] Analytics + usage metering
- [ ] React dashboard UI

### 🔮 v0.4.0
- [ ] WebAssembly runtime (Wasmtime) for instant cold starts
- [ ] Multi-region deployment (Fly.io-style)
- [ ] BGP Anycast routing
- [ ] WireGuard mesh networking between nodes
- [ ] Build output API (standardized artifact format)
- [ ] Incremental Static Regeneration (ISR)

## License

MIT
