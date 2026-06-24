use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::ev;
use super::components::*;
use crate::icons;
use crate::app::{Route, navigate};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EnvVarResponse {
    pub id: String,
    pub key: String,
    pub value: String,
    pub environment: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DomainResponse {
    pub id: String,
    pub domain: String,
    pub verified: bool,
}

#[derive(Debug, serde::Serialize)]
struct SetEnvVarRequest {
    key: String,
    value: String,
    environment: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct AddDomainRequest {
    domain: String,
}

#[component]
pub fn SettingsPage(project_id: String) -> impl IntoView {
    let (project, set_project) = signal(Option::<ProjectResponse>::None);
    let (env_vars, set_env_vars) = signal(Vec::<EnvVarResponse>::new());
    let (domains, set_domains) = signal(Vec::<DomainResponse>::new());
    let (error, set_error) = signal(Option::<String>::None);

    let pid = RwSignal::new(project_id);

    // env var form state
    let (ev_key, set_ev_key) = signal(String::new());
    let (ev_value, set_ev_value) = signal(String::new());
    let (ev_env, set_ev_env) = signal("production".to_string());

    // domain form state
    let (domain_input, set_domain_input) = signal(String::new());

    // Initial load
    {
        let pid = pid.get();
        spawn_local(async move {
            match api_get::<ProjectResponse>(&format!("/projects/{}", pid)).await {
                Ok(p) => set_project.set(Some(p)),
                Err(e) => set_error.set(Some(e)),
            }
            match api_get::<Vec<EnvVarResponse>>(&format!("/projects/{}/env", pid)).await {
                Ok(v) => set_env_vars.set(v),
                Err(_) => {}
            }
            match api_get::<Vec<DomainResponse>>(&format!("/projects/{}/domains", pid)).await {
                Ok(d) => set_domains.set(d),
                Err(_) => {}
            }
        });
    }

    let add_env_var = move || {
        let key = ev_key.get();
        let value = ev_value.get();
        let env = ev_env.get();
        if key.is_empty() || value.is_empty() { return; }
        let pid = pid.get();
        spawn_local(async move {
            let req = SetEnvVarRequest {
                key: key.clone(),
                value: value.clone(),
                environment: Some(env.clone()),
            };
            match api_post::<EnvVarResponse, _>(&format!("/projects/{}/env", pid), &req).await {
                Ok(v) => {
                    set_ev_key.set(String::new());
                    set_ev_value.set(String::new());
                    set_env_vars.update(|vars| vars.push(v));
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let add_domain = move || {
        let domain = domain_input.get();
        if domain.is_empty() { return; }
        let pid = pid.get();
        spawn_local(async move {
            let req = AddDomainRequest { domain: domain.clone() };
            match api_post::<DomainResponse, _>(&format!("/projects/{}/domains", pid), &req).await {
                Ok(d) => {
                    set_domain_input.set(String::new());
                    set_domains.update(|ds| ds.push(d));
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let delete_project = move || {
        let pid = pid.get();
        spawn_local(async move {
            let _ = api_delete(&format!("/projects/{}", pid)).await;
            navigate(Route::Projects);
        });
    };

    view! {
        <div>
            <h1 class="headline-md" style="margin-bottom: 24px;">"Settings"</h1>

            {move || error.get().map(|e| view! {
                <div class="card" style="margin-bottom: 16px; border-color: var(--error);">
                    <span class="body-md" style="color: var(--error);">{e}</span>
                </div>
            })}

            // Project info
            <div class="card" style="margin-bottom: 24px;">
                <h2 class="headline-sm" style="margin-bottom: 20px;">"Project"</h2>
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
                    {move || project.get().map(|p| view! {
                        <>
                            <div>
                                <label class="form-label">"Name"</label>
                                <input class="form-input" type="text" value={p.name} readonly=true />
                            </div>
                            <div>
                                <label class="form-label">"Production branch"</label>
                                <input class="form-input" type="text" value={p.production_branch} readonly=true />
                            </div>
                            <div>
                                <label class="form-label">"Repository"</label>
                                <input class="form-input" type="text" value={p.github_repo_full_name} readonly=true />
                            </div>
                        </>
                    })}
                </div>
            </div>

            // Custom domains
            <div class="card" style="margin-bottom: 24px;">
                <h2 class="headline-sm" style="margin-bottom: 20px;">"Custom Domains"</h2>
                <div style="display: flex; gap: 12px; margin-bottom: 16px;">
                    <input class="form-input" type="text" placeholder="myapp.com"
                        style="flex: 1;"
                        prop:value=domain_input
                        on:input=move |ev| set_domain_input.set(event_target_value(&ev)) />
                    <button class="btn btn-primary btn-sm" on:click=move |_| add_domain()>
                        <span style="display: flex; width: 16px; height: 16px;">{icons::IconPlus()}</span>
                        "Add"
                    </button>
                </div>
                {move || {
                    let ds = domains.get();
                    if ds.is_empty() {
                        view! {
                            <div class="body-sm" style="color: #6b7280; padding: 16px 0;">
                                "No custom domains configured."
                            </div>
                        }.into_any()
                    } else {
                        ds.into_iter().map(|d| {
                            let d_del = d.domain.clone();
                            let pid_val = pid.get();
                            view! {
                                <div style="display: flex; align-items: center; justify-content: space-between; padding: 12px 0; border-bottom: 1px solid var(--border);">
                                    <div style="display: flex; align-items: center; gap: 12px;">
                                        <span class="body-md">{d.domain.clone()}</span>
                                        {if d.verified {
                                            view! {
                                                <span style="display: inline-flex; align-items: center; gap: 4px; font-family: var(--font-mono); font-size: 11px; color: var(--primary); text-transform: uppercase;">
                                                    <span style="display: flex; width: 12px; height: 12px;">{icons::IconCheck()}</span>
                                                    "Verified"
                                                </span>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <span class="label-sm" style="color: #9ca3af;">"Pending"</span>
                                            }.into_any()
                                        }}
                                    </div>
                                    <button class="btn btn-danger btn-sm"
                                        on:click=move |ev: ev::MouseEvent| {
                                            ev.stop_propagation();
                                            let d_del_api = d_del.clone();
                                            let d_del_filter = d_del.clone();
                                            let pid = pid_val.clone();
                                            spawn_local(async move {
                                                let _ = api_delete(&format!("/projects/{}/domains/{}", pid, d_del_api)).await;
                                                set_domains.update(|ds| ds.retain(|x| x.domain != d_del_filter));
                                            });
                                        }>
                                        <span style="display: flex; width: 16px; height: 16px;">{icons::IconTrash()}</span>
                                        "Remove"
                                    </button>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>

            // Environment variables
            <div class="card" style="margin-bottom: 24px;">
                <h2 class="headline-sm" style="margin-bottom: 20px;">"Environment Variables"</h2>
                <div style="display: grid; grid-template-columns: 1fr 2fr 140px auto; gap: 12px; margin-bottom: 16px;">
                    <input class="form-input" type="text" placeholder="KEY"
                        prop:value=ev_key
                        on:input=move |ev| set_ev_key.set(event_target_value(&ev)) />
                    <input class="form-input" type="text" placeholder="value"
                        prop:value=ev_value
                        on:input=move |ev| set_ev_value.set(event_target_value(&ev)) />
                    <select class="form-input" style="height: 48px;"
                        prop:value=ev_env
                        on:change=move |ev| set_ev_env.set(event_target_value(&ev))>
                        <option value="production">"Production"</option>
                        <option value="preview">"Preview"</option>
                    </select>
                    <button class="btn btn-primary btn-sm" on:click=move |_| add_env_var()>
                        <span style="display: flex; width: 16px; height: 16px;">{icons::IconPlus()}</span>
                        "Add"
                    </button>
                </div>
                {move || {
                    let vars = env_vars.get();
                    if vars.is_empty() {
                        view! {
                            <div class="body-sm" style="color: #6b7280; padding: 16px 0;">
                                "No environment variables set."
                            </div>
                        }.into_any()
                    } else {
                        vars.into_iter().map(|v| {
                            let k_del = v.key.clone();
                            let pid_val = pid.get();
                            view! {
                                <div style="display: flex; align-items: center; justify-content: space-between; padding: 12px 0; border-bottom: 1px solid var(--border);">
                                    <div style="display: flex; align-items: center; gap: 12px; flex: 1;">
                                        <code style="font-family: var(--font-mono); font-size: 13px; color: var(--primary);">{v.key.clone()}</code>
                                        <span class="body-sm" style="color: #6b7280;">"="</span>
                                        <code style="font-family: var(--font-mono); font-size: 13px; color: var(--on-surface);">{"•".repeat(v.value.len())}</code>
                                        <span class="label-sm" style="background: var(--neutral); color: var(--on-surface); padding: 2px 8px;">{v.environment.clone()}</span>
                                    </div>
                                    <button class="btn btn-danger btn-sm"
                                        on:click=move |ev: ev::MouseEvent| {
                                            ev.stop_propagation();
                                            let k_del_api = k_del.clone();
                                            let k_del_filter = k_del.clone();
                                            let pid = pid_val.clone();
                                            spawn_local(async move {
                                                let _ = api_delete(&format!("/projects/{}/env/{}", pid, k_del_api)).await;
                                                set_env_vars.update(|vars| vars.retain(|x| x.key != k_del_filter));
                                            });
                                        }>
                                        <span style="display: flex; width: 16px; height: 16px;">{icons::IconTrash()}</span>
                                    </button>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>

            // Danger zone
            <div style="border: 1px solid var(--error); border-radius: 8px; padding: 24px; background: #fef2f2;">
                <h2 class="headline-sm" style="color: var(--error); margin-bottom: 12px;">"Danger Zone"</h2>
                <p class="body-sm" style="color: #6b7280; margin-bottom: 16px;">
                    "Deleting this project will remove all deployments, environment variables, and custom domains. This cannot be undone."
                </p>
                <button class="btn btn-danger btn-sm"
                    on:click=move |_| {
                        if web_sys::window().unwrap()
                            .confirm_with_message("Delete this project and all associated data? This cannot be undone.")
                            .unwrap_or(false) {
                            delete_project();
                        }
                    }>
                    <span style="display: flex; width: 16px; height: 16px;">{icons::IconTrash()}</span>
                    "Delete Project"
                </button>
            </div>
        </div>
    }
}
