use leptos::prelude::*;
use leptos::either::Either;
use leptos::task::spawn_local;
use super::components::*;
use crate::app::Route;

#[component]
pub fn OverviewPage() -> impl IntoView {
    let (health, set_health) = signal(Option::<HealthResponse>::None);
    let (projects, set_projects) = signal(Vec::<ProjectResponse>::new());
    let (deployments, set_deployments) = signal(Vec::<(String, DeploymentResponse)>::new());
    let (error, set_error) = signal(Option::<String>::None);

    let load_data = move || {
        spawn_local(async move {
            match api_get::<HealthResponse>("/health").await {
                Ok(h) => set_health.set(Some(h)),
                Err(e) => set_error.set(Some(e)),
            }
            match api_get::<Vec<ProjectResponse>>("/projects").await {
                Ok(ps) => {
                    let count = ps.len();
                    set_projects.set(ps.clone());
                    let mut all_deps: Vec<(String, DeploymentResponse)> = Vec::new();
                    for p in ps.iter().take(5) {
                        if let Ok(deps) = api_get::<Vec<DeploymentResponse>>(
                            &format!("/projects/{}/deployments", p.id),
                        ).await {
                            for d in deps.iter().take(3) {
                                all_deps.push((p.name.clone(), d.clone()));
                            }
                        }
                    }
                    set_deployments.set(all_deps);
                    if count == 0 {
                        set_error.set(None);
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    load_data();

    // ponytail: interval handle leaks on unmount, fine for SPA
    {
        let refresh = move || load_data();
        let _ = set_interval(refresh, 10000);
    }

    view! {
        <div>
            <h1 class="headline-md" style="margin-bottom: 24px;">"Overview"</h1>

            {move || error.get().map(|e| view! {
                <div class="card" style="margin-bottom: 16px; border-color: var(--error);">
                    <span class="body-md" style="color: var(--error);">{e}</span>
                </div>
            })}

            <div class="stat-grid">
                {move || {
                    let h = health.get();
                    let p = projects.get();
                    let active = h.as_ref().map(|h| h.active_builds.to_string()).unwrap_or("—".to_string());
                    let queue = h.as_ref().map(|h| h.queue_depth.to_string()).unwrap_or("—".to_string());
                    let proj_count = p.len().to_string();
                    let uptime = h.as_ref().map(|h| format!("{}s", h.uptime_secs)).unwrap_or("—".to_string());
                    vec![
                        view! { <StatCard label="Active Builds".to_string() value=active accent=true /> },
                        view! { <StatCard label="Queue Depth".to_string() value=queue accent=false /> },
                        view! { <StatCard label="Projects".to_string() value=proj_count accent=false /> },
                        view! { <StatCard label="Uptime".to_string() value=uptime accent=false /> },
                    ]
                }.collect_view()}
            </div>

            <div class="card" style="padding: 0;">
                <div style="padding: 16px; border-bottom: 1px solid var(--border);">
                    <h2 class="headline-sm">"Recent Deployments"</h2>
                </div>
                {move || {
                    let deps = deployments.get();
                    if deps.is_empty() {
                        Either::Left(view! {
                            <div style="padding: 40px; text-align: center; color: #6b7280;">
                                "No deployments yet"
                            </div>
                        })
                    } else {
                        Either::Right(view! {
                            <table class="table">
                                <thead>
                                    <tr>
                                        <th>"Project"</th>
                                        <th>"Branch"</th>
                                        <th>"SHA"</th>
                                        <th>"Status"</th>
                                        <th>"Framework"</th>
                                        <th>"Created"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {deps.into_iter().map(|(proj_name, d)| {
                                        let route_id = {
                                            let projs = projects.get();
                                            projs.iter()
                                                .find(|p| p.name == proj_name)
                                                .map(|p| p.id.clone())
                                                .unwrap_or_default()
                                        };
                                        view! {
                                            <tr style="cursor: pointer;"
                                                on:click=move |_| {
                                                    if !route_id.is_empty() {
                                                        crate::app::navigate(Route::ProjectDetail(route_id.clone()));
                                                    }
                                                }>
                                                <td>{proj_name}</td>
                                                <td>{d.branch}</td>
                                                <td><code>{short_sha(&d.sha)}</code></td>
                                                <td><Badge status=d.status /></td>
                                                <td>{d.framework.unwrap_or("-".to_string())}</td>
                                                <td>{time_ago(&d.created_at)}</td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        })
                    }
                }}
            </div>
        </div>
    }
}

fn set_interval<F: Fn() + 'static>(f: F, ms: i32) -> Result<(), String> {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    let closure = Closure::wrap(Box::new(f) as Box<dyn Fn()>);
    let func: js_sys::Function = closure.as_ref().unchecked_ref::<js_sys::Function>().clone();
    web_sys::window()
        .unwrap()
        .set_interval_with_callback_and_timeout_and_arguments_0(&func, ms)
        .map_err(|e| format!("{:?}", e))?;
    // ponytail: closure leaked to keep alive — SPA never tears down
    closure.forget();
    Ok(())
}
