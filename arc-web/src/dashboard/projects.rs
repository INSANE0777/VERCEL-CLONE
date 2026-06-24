use leptos::prelude::*;
use leptos::either::Either;
use leptos::task::spawn_local;
use leptos::ev;
use super::components::*;
use crate::app::{Route, navigate};

#[component]
pub fn ProjectsPage() -> impl IntoView {
    let (projects, set_projects) = signal(Vec::<ProjectResponse>::new());
    let (show_modal, set_show_modal) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (form_name, set_form_name) = signal(String::new());
    let (form_repo, set_form_repo) = signal(String::new());
    let (form_branch, set_form_branch) = signal(String::new());
    let (creating, set_creating) = signal(false);

    let load = move || {
        spawn_local(async move {
            match api_get::<Vec<ProjectResponse>>("/projects").await {
                Ok(ps) => set_projects.set(ps),
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    load();

    let do_create = move || {
        let name = form_name.get();
        let repo = form_repo.get();
        let branch = form_branch.get();
        if name.is_empty() || repo.is_empty() {
            set_error.set(Some("Name and repo are required".to_string()));
            return;
        }
        set_creating.set(true);
        spawn_local(async move {
            let req = CreateProjectRequest {
                name: name.clone(),
                github_repo_full_name: repo.clone(),
                production_branch: if branch.is_empty() { None } else { Some(branch) },
            };
            match api_post::<ProjectResponse, _>("/projects", &req).await {
                Ok(_) => {
                    set_show_modal.set(false);
                    set_form_name.set(String::new());
                    set_form_repo.set(String::new());
                    set_form_branch.set(String::new());
                    set_error.set(None);
                    set_creating.set(false);
                    load();
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_creating.set(false);
                }
            }
        });
    };

    let delete_project = move |id: String, _name: String| {
        spawn_local(async move {
            let _ = api_delete(&format!("/projects/{}", id)).await;
            load();
        });
    };

    view! {
        <div>
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
                <h1 class="headline-md">"Projects"</h1>
                <button class="btn btn-primary btn-sm" on:click=move |_| set_show_modal.set(true)>
                    "+ New Project"
                </button>
            </div>

            {move || error.get().map(|e| view! {
                <div class="card" style="margin-bottom: 16px; border-color: var(--error);">
                    <span class="body-md" style="color: var(--error);">{e}</span>
                </div>
            })}

            {move || {
                let ps = projects.get();
                if ps.is_empty() {
                    Either::Left(view! {
                        <div class="card" style="text-align: center; padding: 40px; color: #6b7280;">
                            "No projects yet. Create one to get started."
                        </div>
                    })
                } else {
                    Either::Right(view! {
                        <div style="display: flex; flex-direction: column; gap: 12px;">
                            {ps.into_iter().map(|p| {
                                let pid_nav = p.id.clone();
                                let pid_del = p.id.clone();
                                let _pname = p.name.clone();
                                let pname_del = p.name.clone();
                                let latest_status = p.latest_deployment.as_ref()
                                    .map(|d| d.status.clone());
                                view! {
                                    <div class="card" style="cursor: pointer; display: flex; justify-content: space-between; align-items: center;"
                                        on:click=move |_| navigate(Route::ProjectDetail(pid_nav.clone()))>
                                        <div>
                                            <div class="headline-sm">{p.name}</div>
                                            <div class="body-sm" style="color: #6b7280; margin-top: 4px;">
                                                {format!("{} · {}", p.github_repo_full_name, p.production_branch)}
                                            </div>
                                        </div>
                                        <div style="display: flex; align-items: center; gap: 12px;">
                                            {match latest_status {
                                                Some(s) => Either::Left(view! { <Badge status=s /> }),
                                                None => Either::Right(view! {
                                                    <span class="body-sm" style="color: #6b7280;">"No deployments"</span>
                                                }),
                                            }}
                                            <button class="btn btn-danger btn-sm"
                                                on:click=move |ev: ev::MouseEvent| {
                                                    ev.stop_propagation();
                                                    if web_sys::window().unwrap()
                                                        .confirm_with_message(
                                                            &format!("Delete project \"{}\" and all its deployments?", pname_del)
                                                        ).unwrap_or(false) {
                                                        delete_project(pid_del.clone(), pname_del.clone());
                                                    }
                                                }>
                                                "Delete"
                                            </button>
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    })
                }
            }}

            // Create modal
            {move || show_modal.get().then(|| ()).map(|_| view! {
                <div class="modal-overlay" on:click=move |_| set_show_modal.set(false)>
                    <div class="modal" on:click=move |ev: ev::MouseEvent| ev.stop_propagation()>
                        <h2 class="headline-sm" style="margin-bottom: 24px;">"Create Project"</h2>
                        <div style="margin-bottom: 16px;">
                            <label class="form-label">"Project name"</label>
                            <input class="form-input" type="text"
                                placeholder="my-app"
                                prop:value=form_name
                                on:input=move |ev| set_form_name.set(event_target_value(&ev)) />
                        </div>
                        <div style="margin-bottom: 16px;">
                            <label class="form-label">"GitHub repo (owner/repo)"</label>
                            <input class="form-input" type="text"
                                placeholder="owner/repo"
                                prop:value=form_repo
                                on:input=move |ev| set_form_repo.set(event_target_value(&ev)) />
                        </div>
                        <div style="margin-bottom: 24px;">
                            <label class="form-label">"Production branch"</label>
                            <input class="form-input" type="text"
                                placeholder="main"
                                prop:value=form_branch
                                on:input=move |ev| set_form_branch.set(event_target_value(&ev)) />
                        </div>
                        <div style="display: flex; gap: 12px; justify-content: flex-end;">
                            <button class="btn btn-secondary btn-sm" on:click=move |_| set_show_modal.set(false)>
                                "Cancel"
                            </button>
                            <button class="btn btn-primary btn-sm" disabled=creating
                                on:click=move |_| do_create()>
                                {move || if creating.get() { "Creating..." } else { "Create" }}
                            </button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}
