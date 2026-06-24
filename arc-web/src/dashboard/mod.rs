pub mod components;
pub mod overview;
pub mod projects;
pub mod project_detail;
pub mod analytics;
pub mod settings;

use leptos::prelude::*;
use crate::app::{use_app_state, navigate, Route};
use crate::icons;

#[component]
pub fn DashboardShell() -> impl IntoView {
    let state = use_app_state();

    view! {
        <div class="dash-layout">
            <aside class="dash-sidebar">
                <div class="sidebar-logo">"◆ ARC"</div>
                <div class="sidebar-link" class:active=move || state.route.get() == Route::Overview
                    on:click=move |_| state.route.set(Route::Overview)>
                    {icons::IconHome()}
                    <span>"Overview"</span>
                </div>
                <div class="sidebar-link" class:active=move || state.route.get() == Route::Projects
                    on:click=move |_| state.route.set(Route::Projects)>
                    {icons::IconFolder()}
                    <span>"Projects"</span>
                </div>
                <div class="sidebar-link" class:active=move || state.route.get() == Route::Analytics
                    on:click=move |_| state.route.set(Route::Analytics)>
                    {icons::IconChart()}
                    <span>"Analytics"</span>
                </div>
                <div class="sidebar-link" on:click=move |_| navigate(Route::Landing)>
                    {icons::IconArrow()}
                    <span>"Back to site"</span>
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
                    Route::Settings(id) => {
                        settings::SettingsPage(
                            settings::SettingsPageProps { project_id: id }
                        ).into_any()
                    }
                    Route::Landing => view! { "Redirecting..." }.into_any(),
                }}
            </main>
        </div>
    }
}
