use leptos::prelude::*;
use leptos::either::Either;
use leptos::task::spawn_local;
use super::components::*;

#[component]
pub fn AnalyticsPage() -> impl IntoView {
    let (summary, set_summary) = signal(Option::<AnalyticsSummary>::None);
    let (error, set_error) = signal(Option::<String>::None);

    let load = move || {
        spawn_local(async move {
            match api_get::<AnalyticsSummary>("/analytics/summary").await {
                Ok(s) => set_summary.set(Some(s)),
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    load();

    view! {
        <div>
            <h1 class="headline-md" style="margin-bottom: 24px;">"Analytics"</h1>

            {move || error.get().map(|e| view! {
                <div class="card" style="margin-bottom: 16px; border-color: var(--error);">
                    <span class="body-md" style="color: var(--error);">{e}</span>
                </div>
            })}

            // Stat cards
            <div class="stat-grid">
                {move || {
                    let s = summary.get();
                    let projects = s.as_ref().map(|s| s.total_projects.to_string()).unwrap_or("—".to_string());
                    let deploys = s.as_ref().map(|s| s.total_deployments.to_string()).unwrap_or("—".to_string());
                    let success = s.as_ref().map(|s| format!("{}%", s.success_rate)).unwrap_or("—".to_string());
                    let avg = s.as_ref().map(|s| format!("{}s", s.avg_build_duration_secs)).unwrap_or("—".to_string());
                    vec![
                        view! { <StatCard label="Projects".to_string() value=projects accent=false /> },
                        view! { <StatCard label="Total Deploys".to_string() value=deploys accent=false /> },
                        view! { <StatCard label="Success Rate".to_string() value=success accent=true /> },
                        view! { <StatCard label="Avg Build Time".to_string() value=avg accent=false /> },
                    ]
                }.collect_view()}
            </div>

            // Charts
            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
                // 7-day bar chart
                <div class="card">
                    <h2 class="headline-sm" style="margin-bottom: 16px;">"Deploys (Last 7 Days)"</h2>
                    {move || {
                        let s = summary.get();
                        match s {
                            None => Either::Left(view! { <div class="body-sm" style="color: #6b7280;">"Loading..."</div> }),
                            Some(s) if s.deploys_last_7_days.is_empty() => Either::Left(view! {
                                <div class="body-sm" style="color: #6b7280; text-align: center; padding: 40px;">
                                    "No data yet"
                                </div>
                            }),
                            Some(s) => {
                                let max_count = s.deploys_last_7_days.iter().map(|d| d.count).max().unwrap_or(1);
                                Either::Right(view! {
                                    <div class="chart">
                                        {s.deploys_last_7_days.iter().map(|d| {
                                            let height_pct = (d.count as f64 / max_count as f64) * 100.0;
                                            let label = if d.date.len() >= 5 { d.date[5..].to_string() } else { d.date.clone() };
                                            view! {
                                                <div style="display: flex; flex-direction: column; align-items: center; flex: 1;">
                                                    <div class="bar" style={format!("height: {}%", height_pct)}>
                                                    </div>
                                                    <div class="bar-label">{label}</div>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                })
                            }
                        }
                    }}
                </div>

                // Framework distribution
                <div class="card">
                    <h2 class="headline-sm" style="margin-bottom: 16px;">"Framework Distribution"</h2>
                    {move || {
                        let s = summary.get();
                        match s {
                            None => Either::Left(view! { <div class="body-sm" style="color: #6b7280;">"Loading..."</div> }),
                            Some(s) if s.frameworks.is_empty() => Either::Left(view! {
                                <div class="body-sm" style="color: #6b7280; text-align: center; padding: 40px;">
                                    "No data yet"
                                </div>
                            }),
                            Some(s) => {
                                let max_fw = s.frameworks.iter().map(|f| f.count).max().unwrap_or(1);
                                Either::Right(view! {
                                    <div>
                                        {s.frameworks.iter().map(|f| {
                                            let width_pct = (f.count as f64 / max_fw as f64) * 100.0;
                                            view! {
                                                <div class="fw-row">
                                                    <div class="fw-name">{f.framework.clone()}</div>
                                                    <div class="fw-bar">
                                                        <div class="fw-fill" style={format!("width: {}%", width_pct)}></div>
                                                    </div>
                                                    <div class="fw-count">{f.count.to_string()}</div>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                })
                            }
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
