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
    use_context::<AppState>().unwrap_or(AppState { 
        route: RwSignal::new(Route::Landing),
        api_base: "/api".to_string() 
    })
}

// Global fallback route — used by navigate() when context isn't available
use std::cell::RefCell;
thread_local! {
    static GLOBAL_ROUTE: RefCell<Option<RwSignal<Route>>> = RefCell::new(None);
}

pub fn navigate(route: Route) {
    // Try context first
    if let Some(state) = use_context::<AppState>() {
        state.route.set(route);
        return;
    }
    // Fallback to global
    GLOBAL_ROUTE.with(|r| {
        if let Some(sig) = r.borrow().as_ref() {
            sig.set(route);
        }
    });
}

#[component]
pub fn App() -> impl IntoView {
    provide_app_state();
    let state = use_app_state();
    
    // Also store in global for navigate() fallback
    GLOBAL_ROUTE.with(|r| *r.borrow_mut() = Some(state.route));

    view! {
        <style>{STYLES}</style>
        {move || match state.route.get() {
            Route::Landing => Either::Left(landing::LandingPage()),
            _ => Either::Right(dashboard::DashboardShell()),
        }}
    }
}
