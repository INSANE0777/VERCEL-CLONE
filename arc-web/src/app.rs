use leptos::prelude::*;
use leptos::either::Either;

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
    pub route: RwSignal<Route>,
    pub api_base: String,
}

pub fn provide_app_state() {
    let route = RwSignal::new(Route::Landing);
    let api_base = "/api".to_string();
    provide_context(AppState { route, api_base });
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>().expect("AppState not provided")
}

#[component]
pub fn App() -> impl IntoView {
    provide_app_state();
    let state = use_app_state();

    view! {
        <style>{STYLES}</style>
        {move || match state.route.get() {
            Route::Landing => Either::Left(landing::LandingPage()),
            _ => Either::Right(dashboard::DashboardShell()),
        }}
    }
}

pub fn navigate(route: Route) {
    let state = use_app_state();
    state.route.set(route);
}
