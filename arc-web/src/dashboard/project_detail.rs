use std::rc::Rc;
use leptos::prelude::*;
use leptos::either::Either;
use leptos::task::spawn_local;
use leptos::ev;
use super::components::*;
use crate::app::{Route, navigate};
use crate::icons;

#[component]
pub fn ProjectDetailPage(project_id: String) -> impl IntoView {
    let (project, set_project) = signal(Option::<ProjectResponse>::None);
    let (deployments, set_deployments) = signal(Vec::<DeploymentResponse>::new());
    let (analytics, set_analytics) = signal(Option::<ProjectAnalytics>::None);
    let (error, set_error) = signal(Option::<String>::None);
    let (show_logs, set_show_logs) = signal(Option::<String>::None);
    let (log_content, set_log_content) = signal(String::new());

    let pid_rc = Rc::new(project_id);

    let load: Rc<dyn Fn()> = Rc::new({
        let pid_rc = pid_rc.clone();
        let set_project = set_project;
        let set_deployments = set_deployments;
        let set_analytics = set_analytics;
        let set_error = set_error;
        move || {
            let pid = pid_rc.as_ref().clone();
            spawn_local(async move {
                match api_get::<ProjectResponse>(&format!("/projects/{}", pid)).await {
                    Ok(p) => set_project.set(Some(p)),
                    Err(e) => set_error.set(Some(e)),
                }
                match api_get::<Vec<DeploymentResponse>>(
                    &format!("/projects/{}/deployments", pid),
                ).await {
                    Ok(d) => set_deployments.set(d),
                    Err(e) => set_error.set(Some(e)),
                }
                match api_get::<ProjectAnalytics>(
                    &format!("/projects/{}/analytics", pid),
                ).await {
                    Ok(a) => set_analytics.set(Some(a)),
                    Err(_) => {}
                }
            });
        }
    });

    load();

    let deploy_now = {
        let pid_rc = pid_rc.clone();
        let load = load.clone();
        move || {
            let pid = pid_rc.as_ref().clone();
            let load = load.clone();
            spawn_local(async move {
                match api_post::<serde_json::Value, _>(
                    &format!("/projects/{}/deploy", pid),
                    &serde_json::json!({}),
                ).await {
                    Ok(_) => load(),
                    Err(e) => set_error.set(Some(e)),
                }
            });
        }
    };

    let delete_project = {
        let pid_rc = pid_rc.clone();
        move || {
            let pid = pid_rc.as_ref().clone();
            spawn_local(async move {
                let _ = api_delete(&format!("/projects/{}", pid)).await;
                navigate(Route::Projects);
            });
        }
    };

    let view_logs = move |dep_id: String| {
        let did = dep_id.clone();
        set_show_logs.set(Some(dep_id));
        set_log_content.set("Loading...".to_string());
        spawn_local(async move {
            match api_get::<LogResponse>(&format!("/deployments/{}/logs", did)).await {
                Ok(data) => set_log_content.set(data.logs),
                Err(e) => set_log_content.set(format!("Error: {}", e)),
            }
        });
    };

    view! {
        <div>
            // Header
            <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px;">
                <div>
                    {move || project.get().map(|p| Either::Left(view! {
                        <>
                            <h1 class="headline-md">{p.name}</h1>
                            <div class="body-sm" style="color: #6b7280; margin-top: 4px;">
                                {format!("{} · branch: {}", p.github_repo_full_name, p.production_branch)}
                            </div>
                        </>
                    })).unwrap_or(Either::Right(view! { <h1 class="headline-md">"Loading..."</h1> }))}
                </div>
                <div style="display: flex; gap: 12px;">
                    <button class="btn btn-primary btn-sm" on:click=move |_| deploy_now()>
                        "Deploy Now"
                    </button>
                    <button class="btn btn-secondary btn-sm"
                        on:click=move |_| navigate(Route::Settings(pid_rc.as_ref().clone()))>
                        <span style="display: flex; width: 16px; height: 16px;">{icons::IconSettings()}</span>
                        "Settings"
                    </button>
                    <button class="btn btn-danger btn-sm" on:click=move |_| {
                        if web_sys::window().unwrap()
                            .confirm_with_message("Delete this project and all deployments?")
                            .unwrap_or(false) {
                            delete_project();
                        }
                    }>
                        <span style="display: flex; width: 16px; height: 16px;">{icons::IconTrash()}</span>
                        "Delete Project"
                    </button>
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="card" style="margin-bottom: 16px; border-color: var(--error);">
                    <span class="body-md" style="color: var(--error);">{e}</span>
                </div>
            })}

            // Stats
            <div class="stat-grid">
                {move || {
                    let a = analytics.get();
                    let total = a.as_ref().map(|a| a.total_deployments.to_string()).unwrap_or("—".to_string());
                    let ready = a.as_ref().map(|a| a.ready.to_string()).unwrap_or("—".to_string());
                    let errors = a.as_ref().map(|a| a.errors.to_string()).unwrap_or("—".to_string());
                    let avg = a.as_ref().map(|a| format!("{}s", a.avg_build_duration_secs)).unwrap_or("—".to_string());
                    vec![
                        view! { <StatCard label="Total Deploys".to_string() value=total accent=false /> },
                        view! { <StatCard label="Ready".to_string() value=ready accent=true /> },
                        view! { <StatCard label="Failed".to_string() value=errors accent=false /> },
                        view! { <StatCard label="Avg Duration".to_string() value=avg accent=false /> },
                    ]
                }.collect_view()}
            </div>

            // Deployments table
            <div class="card" style="padding: 0;">
                <div style="padding: 16px; border-bottom: 1px solid var(--border);">
                    <h2 class="headline-sm">"Deployments"</h2>
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
                                        <th>"SHA"</th>
                                        <th>"Branch"</th>
                                        <th>"Status"</th>
                                        <th>"Framework"</th>
                                        <th>"URL"</th>
                                        <th>"Logs"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {deps.into_iter().map(|d| {
                                        let did = d.id.clone();
                                        let url = d.url.clone();
                                        view! {
                                            <tr>
                                                <td><code>{short_sha(&d.sha)}</code></td>
                                                <td>{d.branch}</td>
                                                <td><Badge status=d.status /></td>
                                                <td>{d.framework.unwrap_or("-".to_string())}</td>
                                                <td>{match url {
                                                    Some(u) => Either::Left(view! {
                                                        <a href={format!("http://{}", u.clone())} target="_blank" style="color: var(--primary);">
                                                            {u.clone()}
                                                        </a>
                                                    }),
                                                    None => Either::Right(view! { "-" }),
                                                }}</td>
                                                <td>
                                                    <button class="btn btn-secondary btn-sm" on:click=move |_| view_logs(did.clone())>
                                                        <span style="display: flex; width: 16px; height: 16px;">{icons::IconTerminal()}</span>
                                                        "View Logs"
                                                    </button>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        })
                    }
                }}
            </div>

            // Log viewer modal
            {move || show_logs.get().map(|dep_id| view! {
                <div class="modal-overlay" on:click=move |_| set_show_logs.set(None)>
                    <div class="modal" style="max-width: 800px; width: 95%;" on:click=move |ev: ev::MouseEvent| ev.stop_propagation()>
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;">
                            <h2 class="headline-sm">
                                {format!("Build Logs — {}", short_sha(&dep_id))}
                            </h2>
                            <button class="btn btn-secondary btn-sm" on:click=move |_| set_show_logs.set(None)>
                                "Close"
                            </button>
                        </div>
                        <div class="terminal" style="max-height: 500px; overflow-y: auto;">
                            {log_content.get()}
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}
