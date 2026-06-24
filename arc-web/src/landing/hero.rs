use leptos::prelude::*;
use crate::app::{navigate, Route};

#[component]
pub fn Hero() -> impl IntoView {
    view! {
        <section style="padding: 104px 24px 80px;">
            <div style="max-width: 1200px; margin: 0 auto;">
                <div class="label-sm anim-fade-in delay-1" style="color: var(--primary-60); margin-bottom: 24px;">
                    "AI-INFRASTRUCTURE.DEPLOYMENT.PLATFORM"
                </div>
                <h1 class="headline-display anim-fade-up delay-2" style="color: var(--secondary); max-width: 800px; margin-bottom: 24px;">
                    "Deploy at the "
                    <span style="color: var(--primary);">"speed of thought."</span>
                </h1>
                <p class="body-lg anim-fade-up delay-3" style="color: #9ca3af; max-width: 560px; margin-bottom: 40px;">
                    "ARC clones your repo, detects your framework, builds in isolated containers, and serves on the edge. No config. No waiting."
                </p>
                <div class="anim-fade-up delay-4" style="display: flex; gap: 16px; flex-wrap: wrap;">
                    <button class="btn btn-primary" on:click=move |_| navigate(Route::Overview)>
                        "Get Started →"
                    </button>
                    <button class="btn btn-secondary-dark" on:click=move |_| navigate(Route::Landing)>
                        "View Docs"
                    </button>
                </div>
            </div>
        </section>
    }
}
