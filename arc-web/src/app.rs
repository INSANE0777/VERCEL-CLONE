use leptos::prelude::*;
use leptos::either::Either;
use std::sync::OnceLock;

use crate::styles::STYLES;
use crate::landing;
use crate::dashboard;

#[derive(Clone, PartialEq)]
pub enum Route {
    Landing,
    Overview,
    Projects,
    ProjectDetail(String),
    Analytics,
}

#[derive(Clone)]
pub struct AppState {
    pub api_base: String,
}

pub fn provide_app_state() {
    provide_context(AppState { api_base: "/api".to_string() });
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>().unwrap_or(AppState { api_base: "/api".to_string() })
}

// Global route signal — created once, accessible from anywhere
static ROUTE: OnceLock<RwSignal<Route>> = OnceLock::new();

pub fn route_signal() -> RwSignal<Route> {
    *ROUTE.get_or_init(|| RwSignal::new(Route::Landing))
}

pub fn navigate(route: Route) {
    route_signal().set(route);
}

#[component]
pub fn App() -> impl IntoView {
    provide_app_state();
    let route = route_signal();

    view! {
        <style>{STYLES}</style>
        {move || match route.get() {
            Route::Landing => Either::Left(landing::LandingPage()),
            _ => Either::Right(dashboard::DashboardShell()),
        }}
    }
}
