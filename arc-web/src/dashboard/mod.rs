pub mod components;
pub mod overview;
pub mod projects;
pub mod project_detail;
pub mod analytics;

use leptos::prelude::*;
use crate::app::{use_app_state, Route};

#[component]
pub fn DashboardShell() -> impl IntoView {
    let state = use_app_state();

    view! {
        <div class="dash-layout">
            <aside class="dash-sidebar">
                <div class="sidebar-logo">"◆ ARC"</div>
                <div class="sidebar-link" class:active=move || state.route.get() == Route::Overview
                    on:click=move |_| state.route.set(Route::Overview)>
                    <span>"▣ Overview"</span>
                </div>
                <div class="sidebar-link" class:active=move || state.route.get() == Route::Projects
                    on:click=move |_| state.route.set(Route::Projects)>
                    <span>"▤ Projects"</span>
                </div>
                <div class="sidebar-link" class:active=move || state.route.get() == Route::Analytics
                    on:click=move |_| state.route.set(Route::Analytics)>
                    <span>"▥ Analytics"</span>
                </div>
                <div class="sidebar-link" on:click=move |_| state.route.set(Route::Landing)>
                    <span>"↩ Back to site"</span>
                </div>
            </aside>
            <main class="dash-content">
                {move || match state.route.get() {
                    Route::Overview => overview::OverviewPage().into_any(),
                    Route::Projects => projects::ProjectsPage().into_any(),
                    Route::ProjectDetail(id) => {
                        project_detail::ProjectDetailPage(
                            project_detail::ProjectDetailPageProps { project_id: id }
                        ).into_any()
                    }
                    Route::Analytics => analytics::AnalyticsPage().into_any(),
                    Route::Landing => view! { "Redirecting..." }.into_any(),
                }}
            </main>
        </div>
    }
}
