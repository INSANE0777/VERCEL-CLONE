pub const STYLES: &str = r#"
:root {
  --primary: #501cbe;
  --primary-60: #b8a3e8;
  --primary-70: #9d7fd9;
  --primary-80: #7c5cc7;
  --primary-90: #501cbe;
  --secondary: #FFFFFF;
  --tertiary: #0A0A0A;
  --surface: #FFFFFF;
  --on-surface: #0A0A0A;
  --neutral: #E5E5E5;
  --border: #E5E7EB;
  --error: #D94B4B;
  --font-display: 'Space Grotesk', system-ui, sans-serif;
  --font-mono: 'JetBrains Mono', ui-monospace, monospace;
}

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
  font-family: var(--font-display);
  background: var(--tertiary);
  color: var(--on-surface);
  -webkit-font-smoothing: antialiased;
}

a { color: inherit; text-decoration: none; }

/* ── Typography ── */
.headline-display { font-size: 48px; font-weight: 500; line-height: 1.1; letter-spacing: -0.02em; }
.headline-lg { font-size: 38px; font-weight: 500; line-height: 1.15; letter-spacing: -0.02em; }
.headline-md { font-size: 31px; font-weight: 500; line-height: 1.2; letter-spacing: -0.01em; }
.headline-sm { font-size: 24px; font-weight: 500; line-height: 1.25; }
.body-lg { font-size: 19.46px; font-weight: 500; line-height: 1.5; }
.body-md { font-size: 16px; font-weight: 400; line-height: 1.55; }
.body-sm { font-size: 14px; font-weight: 400; line-height: 1.5; }
.label-lg { font-family: var(--font-mono); font-size: 16px; font-weight: 500; }
.label-md { font-family: var(--font-mono); font-size: 14px; font-weight: 500; }
.label-sm { font-family: var(--font-mono); font-size: 12px; font-weight: 500; letter-spacing: 0.04em; }

/* ── Buttons ── */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 48px;
  padding: 0 24px;
  border: none;
  border-radius: 0;
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.15s;
}
.btn:hover { opacity: 0.85; }
.btn-primary { background: var(--primary); color: var(--secondary); }
.btn-secondary { background: transparent; color: var(--on-surface); border: 1px solid var(--on-surface); }
.btn-secondary-dark { background: transparent; color: var(--secondary); border: 1px solid var(--secondary); }
.btn-danger { background: transparent; color: var(--error); border: 1px solid var(--error); }
.btn-sm { height: 36px; padding: 0 16px; font-size: 12px; }
.btn:disabled { opacity: 0.4; cursor: not-allowed; }

/* ── Cards ── */
.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
}

/* ── Badges ── */
.badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 10px;
  border-radius: 9999px;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.badge-ready { background: var(--primary); color: var(--secondary); }
.badge-error { background: var(--error); color: var(--secondary); }
.badge-building { background: var(--tertiary); color: var(--secondary); }
.badge-queued { background: var(--neutral); color: var(--on-surface); }

/* ── Stat cards ── */
.stat-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 24px;
}
.stat-value { font-size: 36px; font-weight: 500; color: var(--on-surface); letter-spacing: -0.02em; font-family: var(--font-display); }
.stat-label { font-family: var(--font-mono); font-size: 12px; font-weight: 500; color: #6b7280; text-transform: uppercase; letter-spacing: 0.06em; margin-top: 8px; }
.stat-card.accent .stat-value { color: var(--primary); }

/* ── Tables ── */
.table { width: 100%; border-collapse: collapse; }
.table th {
  text-align: left;
  padding: 10px 12px;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #6b7280;
  border-bottom: 1px solid var(--border);
}
.table td {
  padding: 12px;
  font-size: 14px;
  border-bottom: 1px solid var(--border);
}
.table tr:hover td { background: #fafafa; }

/* ── Forms ── */
.form-input {
  width: 100%;
  height: 48px;
  padding: 0 16px;
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 14px;
  font-family: var(--font-display);
  background: var(--surface);
  color: var(--on-surface);
}
.form-input:focus { outline: none; border-color: var(--primary); }
.form-label { font-family: var(--font-mono); font-size: 12px; font-weight: 500; color: #6b7280; text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: 6px; display: block; }

/* ── Modal ── */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(10,10,10,0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}
.modal {
  background: var(--surface);
  border-radius: 8px;
  padding: 32px;
  width: 90%;
  max-width: 480px;
}

/* ── Layout ── */
.dash-layout { display: flex; min-height: 100vh; background: #f9f9f9; }
.dash-sidebar {
  width: 240px;
  background: var(--tertiary);
  color: var(--secondary);
  padding: 24px 0;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
}
.dash-content { flex: 1; padding: 40px; overflow-y: auto; }

/* ── Sidebar nav ── */
.sidebar-logo { padding: 0 24px 24px; font-family: var(--font-mono); font-size: 18px; font-weight: 600; }
.sidebar-link {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 24px;
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: 500;
  color: #9ca3af;
  cursor: pointer;
  transition: all 0.15s;
}
.sidebar-link:hover { color: var(--secondary); background: rgba(255,255,255,0.05); }
.sidebar-link.active { color: var(--secondary); background: rgba(80,28,190,0.2); border-left: 2px solid var(--primary); }

/* ── Terminal block ── */
.terminal {
  background: var(--tertiary);
  color: #e0e0e0;
  border: 1px solid #1f1f1f;
  border-radius: 8px;
  padding: 24px;
  font-family: var(--font-mono);
  font-size: 14px;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-all;
}
.terminal-prompt { color: var(--primary-60); }
.terminal-arrow { color: var(--primary-70); }
.terminal-success { color: #4ade80; }

/* ── Bar chart ── */
.chart { display: flex; align-items: flex-end; gap: 8px; height: 140px; margin-top: 16px; }
.bar { flex: 1; background: var(--primary); min-height: 2px; transition: height 0.3s; }
.bar:hover { background: var(--primary-80); }
.bar-label { font-family: var(--font-mono); font-size: 10px; color: #6b7280; text-align: center; margin-top: 6px; }

/* ── Framework distribution ── */
.fw-row { display: flex; align-items: center; gap: 12px; margin-bottom: 10px; }
.fw-name { width: 100px; font-size: 14px; font-family: var(--font-mono); }
.fw-bar { flex: 1; height: 8px; background: var(--neutral); border-radius: 0; overflow: hidden; }
.fw-fill { height: 100%; background: var(--primary); }
.fw-count { font-family: var(--font-mono); font-size: 13px; color: #6b7280; width: 40px; text-align: right; }

/* ── Stat grid ── */
.stat-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px; margin-bottom: 24px; }

/* ── Responsive ── */
@media (max-width: 768px) {
  .dash-sidebar { width: 60px; }
  .sidebar-link span { display: none; }
  .sidebar-logo { font-size: 14px; padding: 0 16px 24px; }
  .dash-content { padding: 24px 16px; }
  .headline-display { font-size: 32px; }
  .headline-lg { font-size: 28px; }
}
"#;
