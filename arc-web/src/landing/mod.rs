pub mod hero;
pub mod features;
pub mod how_it_works;
pub mod code_block;
pub mod footer;

use leptos::prelude::*;
use crate::app::navigate;
use crate::app::Route;

#[component]
pub fn LandingPage() -> impl IntoView {
    view! {
        <div style="background: var(--tertiary); color: var(--secondary); min-height: 100vh;">
            // Promo bar
            <div class="anim-fade-in" style="background: #111; border-bottom: 1px solid #1f1f1f; padding: 8px 0;">
                <div style="max-width: 1200px; margin: 0 auto; padding: 0 24px;">
                    <span class="label-sm" style="color: var(--primary-60);">
                        "◆ ARC · Now in alpha · git push to deploy"
                    </span>
                </div>
            </div>

            // Top nav
            <nav class="anim-fade-in delay-1" style="display: flex; align-items: center; justify-content: space-between; max-width: 1200px; margin: 0 auto; padding: 20px 24px;">
                <div class="label-lg" style="font-weight: 600; font-size: 18px;">
                    "◆ ARC"
                </div>
                <div style="display: flex; align-items: center; gap: 40px;">
                    <a class="label-md" style="color: #9ca3af; transition: color 0.2s;" href="#">"Docs"</a>
                    <a class="label-md" style="color: #9ca3af; transition: color 0.2s;" href="#">"Pricing"</a>
                    <a class="label-md" style="color: #9ca3af; transition: color 0.2s;" href="#">"GitHub"</a>
                    <button class="btn btn-primary btn-sm" on:click=move |_| navigate(Route::Overview)>
                        "Get Started"
                    </button>
                </div>
            </nav>

            {hero::Hero()}
            {features::Features()}
            {how_it_works::HowItWorks()}
            {code_block::CodeBlock()}

            // Stats strip
            <div style="border-top: 1px solid #1f1f1f; border-bottom: 1px solid #1f1f1f;">
                <div style="max-width: 1200px; margin: 0 auto; padding: 52px 24px; display: grid; grid-template-columns: repeat(4, 1fr); gap: 24px;">
                    <div class="anim-fade-up delay-1">
                        <div class="headline-sm" style="color: var(--secondary);">"4"</div>
                        <div class="label-sm" style="color: #9ca3af; margin-top: 4px;">"CONCURRENT BUILDS"</div>
                    </div>
                    <div class="anim-fade-up delay-2">
                        <div class="headline-sm" style="color: var(--secondary);">"3-TIER"</div>
                        <div class="label-sm" style="color: #9ca3af; margin-top: 4px;">"ISOLATION"</div>
                    </div>
                    <div class="anim-fade-up delay-3">
                        <div class="headline-sm" style="color: var(--secondary);">"<1s"</div>
                        <div class="label-sm" style="color: #9ca3af; margin-top: 4px;">"EDGE ROUTING"</div>
                    </div>
                    <div class="anim-fade-up delay-4">
                        <div class="headline-sm" style="color: var(--secondary);">"100%"</div>
                        <div class="label-sm" style="color: #9ca3af; margin-top: 4px;">"OPEN SOURCE"</div>
                    </div>
                </div>
            </div>

            // CTA section
            <div class="cta-section" style="background: var(--primary); transition: background 0.3s;">
                <div style="max-width: 1200px; margin: 0 auto; padding: 104px 24px; text-align: center;">
                    <h2 class="headline-display anim-fade-up" style="color: var(--tertiary); margin-bottom: 24px;">
                        "Ready to deploy?"
                    </h2>
                    <button class="btn btn-sm anim-scale-in delay-2" style="background: var(--tertiary); color: var(--secondary);"
                            on:click=move |_| navigate(Route::Overview)>
                        "Get Started →"
                    </button>
                </div>
            </div>

            {footer::Footer()}
        </div>
    }
}
