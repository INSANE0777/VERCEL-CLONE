pub const DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>vercel-clone — Dashboard</title>
<style>
:root {
  --primary: #501cbe;
  --primary-deep: #3d1496;
  --primary-soft: #e8dff7;
  --ink: #171717;
  --body: #4d4d4d;
  --mute: #888888;
  --hairline: #ebebeb;
  --canvas: #ffffff;
  --canvas-soft: #fafafa;
  --canvas-soft-2: #f5f5f5;
  --error: #ee0000;
  --error-soft: #f7d4d6;
  --warning: #f5a623;
  --success: #501cbe;
}
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: Inter, system-ui, -apple-system, sans-serif; background: var(--canvas-soft); color: var(--ink); font-size: 14px; line-height: 1.5; }
a { color: var(--primary); text-decoration: none; }
a:hover { text-decoration: underline; }

/* Nav */
.nav { background: var(--canvas); border-bottom: 1px solid var(--hairline); height: 56px; display: flex; align-items: center; padding: 0 24px; gap: 16px; position: sticky; top: 0; z-index: 100; }
.nav-logo { font-weight: 600; font-size: 16px; color: var(--primary); display: flex; align-items: center; gap: 8px; }
.nav-logo svg { width: 20px; height: 20px; }
.nav-links { display: flex; gap: 4px; flex: 1; }
.nav-link { padding: 6px 12px; border-radius: 6px; color: var(--body); cursor: pointer; font-size: 14px; font-weight: 500; }
.nav-link:hover { background: var(--canvas-soft-2); color: var(--ink); }
.nav-link.active { background: var(--primary-soft); color: var(--primary); }

/* Layout */
.container { max-width: 1200px; margin: 0 auto; padding: 24px; }
.page { display: none; }
.page.active { display: block; }

/* Cards */
.card { background: var(--canvas); border: 1px solid var(--hairline); border-radius: 8px; padding: 20px; margin-bottom: 16px; }
.card-title { font-size: 14px; font-weight: 600; color: var(--ink); margin-bottom: 12px; }

/* Stats grid */
.stats-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 16px; margin-bottom: 24px; }
.stat-card { background: var(--canvas); border: 1px solid var(--hairline); border-radius: 8px; padding: 20px; }
.stat-value { font-size: 32px; font-weight: 600; color: var(--ink); letter-spacing: -1px; }
.stat-label { font-size: 12px; color: var(--mute); text-transform: uppercase; letter-spacing: 0.5px; margin-top: 4px; }
.stat-card.accent .stat-value { color: var(--primary); }

/* Tables */
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th { text-align: left; padding: 8px 12px; background: var(--canvas-soft); border-bottom: 1px solid var(--hairline); font-weight: 500; color: var(--mute); text-transform: uppercase; font-size: 11px; letter-spacing: 0.5px; }
td { padding: 10px 12px; border-bottom: 1px solid var(--hairline); color: var(--ink); }
tr:hover td { background: var(--canvas-soft); }

/* Badges */
.badge { display: inline-flex; align-items: center; padding: 2px 8px; border-radius: 9999px; font-size: 11px; font-weight: 500; }
.badge-ready { background: var(--primary-soft); color: var(--primary); }
.badge-building { background: #fff3e0; color: #e65100; }
.badge-error { background: var(--error-soft); color: var(--error); }
.badge-queued { background: var(--canvas-soft-2); color: var(--mute); }

/* Buttons */
.btn { display: inline-flex; align-items: center; padding: 8px 16px; border-radius: 6px; font-size: 14px; font-weight: 500; border: none; cursor: pointer; }
.btn-primary { background: var(--primary); color: #fff; }
.btn-primary:hover { background: var(--primary-deep); }
.btn-secondary { background: var(--canvas); color: var(--ink); border: 1px solid var(--hairline); }
.btn-sm { padding: 4px 12px; font-size: 13px; }

/* Forms */
.form-row { display: flex; gap: 12px; margin-bottom: 12px; }
.form-input { flex: 1; padding: 0 12px; height: 36px; border: 1px solid var(--hairline); border-radius: 6px; font-size: 14px; background: var(--canvas); color: var(--ink); }
.form-input:focus { outline: none; border-color: var(--primary); }

/* Modal */
.modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.3); display: none; align-items: center; justify-content: center; z-index: 200; }
.modal-overlay.show { display: flex; }
.modal { background: var(--canvas); border-radius: 12px; padding: 24px; width: 90%; max-width: 480px; }
.modal-title { font-size: 18px; font-weight: 600; margin-bottom: 16px; }

/* Bar chart */
.chart { display: flex; align-items: flex-end; gap: 4px; height: 120px; margin-top: 12px; }
.bar { flex: 1; background: var(--primary); border-radius: 4px 4px 0 0; min-height: 2px; position: relative; transition: height 0.3s; }
.bar:hover { background: var(--primary-deep); }
.bar-label { position: absolute; bottom: -20px; left: 50%; transform: translateX(-50%); font-size: 10px; color: var(--mute); white-space: nowrap; }

/* Framework list */
.fw-row { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.fw-name { width: 120px; font-size: 13px; color: var(--ink); }
.fw-bar { flex: 1; height: 8px; background: var(--canvas-soft-2); border-radius: 4px; overflow: hidden; }
.fw-fill { height: 100%; background: var(--primary); border-radius: 4px; }
.fw-count { font-size: 12px; color: var(--mute); width: 30px; text-align: right; }

/* Log viewer */
.logs { background: var(--ink); color: #e0e0e0; border-radius: 8px; padding: 16px; font-family: ui-monospace, Menlo, monospace; font-size: 12px; line-height: 1.6; max-height: 400px; overflow-y: auto; white-space: pre-wrap; word-break: break-all; }

/* Project header */
.project-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 20px; }
.project-header h2 { font-size: 20px; font-weight: 600; }
.project-meta { font-size: 13px; color: var(--mute); margin-top: 4px; }

/* Responsive */
@media (max-width: 600px) {
  .stats-grid { grid-template-columns: repeat(2, 1fr); }
  .nav-links { display: none; }
}
</style>
</head>
<body>

<nav class="nav">
  <div class="nav-logo">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2L2 22h20L12 2z"/></svg>
    vercel-clone
  </div>
  <div class="nav-links">
    <div class="nav-link active" data-page="overview">Overview</div>
    <div class="nav-link" data-page="projects">Projects</div>
    <div class="nav-link" data-page="analytics">Analytics</div>
  </div>
  <button class="btn btn-primary btn-sm" onclick="showCreateModal()">+ New Project</button>
</nav>

<div class="container">
  <!-- Overview Page -->
  <div class="page active" id="page-overview">
    <div class="stats-grid" id="overview-stats"></div>
    <div class="card">
      <div class="card-title">Recent Deployments</div>
      <table>
        <thead><tr><th>Project</th><th>Branch</th><th>SHA</th><th>Status</th><th>Framework</th><th>Created</th></tr></thead>
        <tbody id="overview-deployments"></tbody>
      </table>
    </div>
  </div>

  <!-- Projects Page -->
  <div class="page" id="page-projects">
    <div id="projects-list"></div>
  </div>

  <!-- Analytics Page -->
  <div class="page" id="page-analytics">
    <div class="stats-grid" id="analytics-stats"></div>
    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
      <div class="card">
        <div class="card-title">Deploys (Last 7 Days)</div>
        <div class="chart" id="chart-7day"></div>
      </div>
      <div class="card">
        <div class="card-title">Framework Distribution</div>
        <div id="framework-dist"></div>
      </div>
    </div>
  </div>

  <!-- Project Detail -->
  <div class="page" id="page-project-detail">
    <div class="project-header">
      <div>
        <h2 id="pd-name"></h2>
        <div class="project-meta" id="pd-meta"></div>
      </div>
      <div style="display:flex;gap:8px">
        <button class="btn btn-primary btn-sm" onclick="deployProject()">Deploy Now</button>
        <button class="btn btn-sm" style="background:#1a1a1a;color:#e5484d;border:1px solid #e5484d" onclick="deleteProject()">Delete Project</button>
      </div>
    </div>
    <div class="stats-grid" id="pd-stats"></div>
    <div class="card">
      <div class="card-title">Deployments</div>
      <table>
        <thead><tr><th>SHA</th><th>Branch</th><th>Status</th><th>Framework</th><th>URL</th><th>Logs</th></tr></thead>
        <tbody id="pd-deployments"></tbody>
      </table>
    </div>
  </div>
</div>

<!-- Create Project Modal -->
<div class="modal-overlay" id="create-modal">
  <div class="modal">
    <div class="modal-title">Create Project</div>
    <div class="form-row"><input class="form-input" id="cp-name" placeholder="Project name (e.g. my-app)"></div>
    <div class="form-row"><input class="form-input" id="cp-repo" placeholder="GitHub repo (owner/repo)"></div>
    <div class="form-row"><input class="form-input" id="cp-branch" placeholder="Production branch (default: main)"></div>
    <div style="display: flex; gap: 8px; justify-content: flex-end; margin-top: 16px;">
      <button class="btn btn-secondary" onclick="hideCreateModal()">Cancel</button>
      <button class="btn btn-primary" onclick="createProject()">Create</button>
    </div>
  </div>
</div>

<script>
const API = '/api';

// ── Navigation ──
document.querySelectorAll('.nav-link').forEach(el => {
  el.addEventListener('click', () => {
    document.querySelectorAll('.nav-link').forEach(n => n.classList.remove('active'));
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    el.classList.add('active');
    document.getElementById('page-' + el.dataset.page).classList.add('active');
    if (el.dataset.page === 'overview') loadOverview();
    if (el.dataset.page === 'projects') loadProjects();
    if (el.dataset.page === 'analytics') loadAnalytics();
  });
});

function showPage(pageId) {
  document.querySelectorAll('.nav-link').forEach(n => n.classList.remove('active'));
  document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
  document.getElementById(pageId).classList.add('active');
}

// ── API helpers ──
async function api(path, opts = {}) {
  const res = await fetch(API + path, { headers: {'Content-Type': 'application/json'}, ...opts });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

function timeAgo(iso) {
  const d = new Date(iso);
  const s = Math.floor((Date.now() - d) / 1000);
  if (s < 60) return s + 's ago';
  if (s < 3600) return Math.floor(s/60) + 'm ago';
  if (s < 86400) return Math.floor(s/3600) + 'h ago';
  return Math.floor(s/86400) + 'd ago';
}

function badge(status) {
  return `<span class="badge badge-${status}">${status}</span>`;
}

// ── Overview ──
async function loadOverview() {
  try {
    const [health, projects] = await Promise.all([api('/health'), api('/projects')]);
    document.getElementById('overview-stats').innerHTML = `
      <div class="stat-card"><div class="stat-value">${health.active_builds}</div><div class="stat-label">Active Builds</div></div>
      <div class="stat-card"><div class="stat-value">${health.queue_depth}</div><div class="stat-label">Queue Depth</div></div>
      <div class="stat-card"><div class="stat-value">${projects.length}</div><div class="stat-label">Projects</div></div>
      <div class="stat-card accent"><div class="stat-value">${health.uptime_secs}s</div><div class="stat-label">Uptime</div></div>
    `;
    // Load recent deployments from all projects
    let rows = '';
    for (const p of projects.slice(0, 5)) {
      const deps = await api('/projects/' + p.id + '/deployments');
      for (const d of deps.slice(0, 3)) {
        rows += `<tr style="cursor:pointer" onclick="openProject('${p.id}')">
          <td>${p.name}</td><td>${d.branch}</td><td><code>${d.sha.slice(0,7)}</code></td>
          <td>${badge(d.status)}</td><td>${d.framework || '-'}</td><td>${timeAgo(d.created_at)}</td>
        </tr>`;
      }
    }
    document.getElementById('overview-deployments').innerHTML = rows || '<tr><td colspan="6" style="text-align:center;color:var(--mute)">No deployments yet</td></tr>';
  } catch(e) { console.error(e); }
}

// ── Projects ──
async function loadProjects() {
  const projects = await api('/projects');
  let html = '';
  for (const p of projects) {
    const latest = p.latest_deployment;
    html += `<div class="card" style="cursor:pointer" onclick="openProject('${p.id}')">
      <div style="display:flex;justify-content:space-between;align-items:center">
        <div>
          <div style="font-weight:600;font-size:16px">${p.name}</div>
          <div style="color:var(--mute);font-size:13px">${p.github_repo_full_name} · ${p.production_branch}</div>
        </div>
        <div style="display:flex;align-items:center;gap:12px">
          ${latest ? badge(latest.status) : '<span style="color:var(--mute)">No deployments</span>'}
          <button class="btn btn-sm" style="background:#1a1a1a;color:#e5484d;border:1px solid #e5484d;padding:4px 10px" onclick="event.stopPropagation();deleteProjectById('${p.id}','${p.name}')">Delete</button>
        </div>
      </div>
    </div>`;
  }
  document.getElementById('projects-list').innerHTML = html || '<div style="text-align:center;color:var(--mute);padding:40px">No projects yet. Create one to get started.</div>';
}

// ── Project Detail ──
let currentProjectId = null;

async function openProject(id) {
  currentProjectId = id;
  showPage('page-project-detail');
  const [project, deps, analytics] = await Promise.all([
    api('/projects/' + id),
    api('/projects/' + id + '/deployments'),
    api('/projects/' + id + '/analytics'),
  ]);
  document.getElementById('pd-name').textContent = project.name;
  document.getElementById('pd-meta').textContent = `${project.github_repo_full_name} · branch: ${project.production_branch}`;
  document.getElementById('pd-stats').innerHTML = `
    <div class="stat-card"><div class="stat-value">${analytics.total_deployments}</div><div class="stat-label">Total Deploys</div></div>
    <div class="stat-card accent"><div class="stat-value">${analytics.ready}</div><div class="stat-label">Ready</div></div>
    <div class="stat-card"><div class="stat-value" style="color:var(--error)">${analytics.errors}</div><div class="stat-label">Failed</div></div>
    <div class="stat-card"><div class="stat-value">${analytics.avg_build_duration_secs}s</div><div class="stat-label">Avg Duration</div></div>
  `;
  let rows = '';
  for (const d of deps) {
    rows += `<tr>
      <td><code>${d.sha.slice(0,7)}</code></td><td>${d.branch}</td>
      <td>${badge(d.status)}</td><td>${d.framework || '-'}</td>
      <td>${d.url ? `<a href="http://${d.url}" target="_blank">${d.url}</a>` : '-'}</td>
      <td><button class="btn btn-secondary btn-sm" onclick="event.stopPropagation();viewLogs('${d.id}')">View</button></td>
    </tr>`;
  }
  document.getElementById('pd-deployments').innerHTML = rows || '<tr><td colspan="6" style="text-align:center;color:var(--mute)">No deployments yet</td></tr>';
}

async function deployProject() {
  if (!currentProjectId) return;
  await api('/projects/' + currentProjectId + '/deploy', { method: 'POST', body: '{}' });
  openProject(currentProjectId);
}

async function deleteProject() {
  if (!currentProjectId) return;
  const name = document.getElementById('pd-name').textContent;
  deleteProjectById(currentProjectId, name);
}

async function deleteProjectById(id, name) {
  if (!confirm('Delete project "' + name + '" and all its deployments? This cannot be undone.')) return;
  await api('/projects/' + id, { method: 'DELETE' });
  currentProjectId = null;
  showPage('page-projects');
  document.querySelector('[data-page="projects"]').classList.add('active');
  loadProjects();
}

// ── View Logs (with live WebSocket streaming) ──
let logWebSocket = null;

async function viewLogs(deploymentId) {
  const data = await api('/deployments/' + deploymentId + '/logs');
  const logs = data.logs || 'No logs available';
  const w = window.open('', '_blank', 'width=800,height=600');
  w.document.write(`<title>Build Logs — ${deploymentId.slice(0,8)}</title>
    <style>
      body{background:#171717;color:#e0e0e0;font-family:ui-monospace,Menlo,monospace;font-size:12px;padding:16px;white-space:pre-wrap;word-break:break-all}
      .live{color:#501cbe;font-weight:bold}
    </style>
    <div class="live" id="live-indicator">Live streaming...</div>
    <pre id="log-content">${logs.replace(/</g,'&lt;')}</pre>`);
  
  // Connect WebSocket for live log streaming
  if (logWebSocket) logWebSocket.close();
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  logWebSocket = new WebSocket(proto + '://' + location.host + '/api/deployments/' + deploymentId + '/status/stream');
  logWebSocket.onmessage = function(e) {
    try {
      const update = JSON.parse(e.data);
      if (update.message && update.message !== 'current status' && update.message !== 'Build started') {
        const pre = w.document.getElementById('log-content');
        pre.textContent += update.message;
        w.scrollTo(0, w.document.body.scrollHeight);
      }
      if (update.status === 'ready' || update.status === 'error') {
        w.document.getElementById('live-indicator').textContent = 'Build ' + update.status;
        logWebSocket.close();
      }
    } catch(err) {}
  };
  logWebSocket.onclose = function() {
    if (w.document.getElementById('live-indicator'))
      w.document.getElementById('live-indicator').textContent = 'Stream ended';
  };
}

// ── Analytics ──
async function loadAnalytics() {
  const data = await api('/analytics/summary');
  document.getElementById('analytics-stats').innerHTML = `
    <div class="stat-card"><div class="stat-value">${data.total_projects}</div><div class="stat-label">Projects</div></div>
    <div class="stat-card"><div class="stat-value">${data.total_deployments}</div><div class="stat-label">Total Deploys</div></div>
    <div class="stat-card accent"><div class="stat-value">${data.success_rate}%</div><div class="stat-label">Success Rate</div></div>
    <div class="stat-card"><div class="stat-value">${data.avg_build_duration_secs}s</div><div class="stat-label">Avg Build Time</div></div>
  `;
  // 7-day chart
  const chartData = data.deploys_last_7_days || [];
  const maxCount = Math.max(...chartData.map(d => d.count), 1);
  let chartHtml = '';
  for (const d of chartData) {
    const h = (d.count / maxCount) * 100;
    chartHtml += `<div class="bar" style="height:${h}%" title="${d.date}: ${d.count} deploys"><div class="bar-label">${d.date.slice(5)}</div></div>`;
  }
  document.getElementById('chart-7day').innerHTML = chartHtml || '<div style="color:var(--mute);text-align:center;width:100%">No data yet</div>';
  // Framework distribution
  const fws = data.frameworks || [];
  const maxFw = Math.max(...fws.map(f => f.count), 1);
  let fwHtml = '';
  for (const f of fws) {
    fwHtml += `<div class="fw-row"><div class="fw-name">${f.framework}</div><div class="fw-bar"><div class="fw-fill" style="width:${(f.count/maxFw)*100}%"></div></div><div class="fw-count">${f.count}</div></div>`;
  }
  document.getElementById('framework-dist').innerHTML = fwHtml || '<div style="color:var(--mute);text-align:center">No data yet</div>';
}

// ── Create Project ──
function showCreateModal() { document.getElementById('create-modal').classList.add('show'); }
function hideCreateModal() { document.getElementById('create-modal').classList.remove('show'); }

async function createProject() {
  const name = document.getElementById('cp-name').value.trim();
  const repo = document.getElementById('cp-repo').value.trim();
  const branch = document.getElementById('cp-branch').value.trim() || 'main';
  if (!name || !repo) return alert('Name and repo are required');
  await api('/projects', { method: 'POST', body: JSON.stringify({ name, github_repo_full_name: repo, production_branch: branch }) });
  hideCreateModal();
  document.getElementById('cp-name').value = '';
  document.getElementById('cp-repo').value = '';
  document.getElementById('cp-branch').value = '';
  loadProjects();
  showPage('page-projects');
  document.querySelector('[data-page="projects"]').classList.add('active');
}

// ── Init ──
loadOverview();

// Refresh overview every 10s
setInterval(() => {
  if (document.getElementById('page-overview').classList.contains('active')) loadOverview();
}, 10000);
</script>
</body>
</html>"##;
