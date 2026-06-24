pub mod hero;
pub mod features;
pub mod how_it_works;
pub mod code_block;
pub mod footer;
pub mod pricing;
pub mod faq;
pub mod comparison;
pub mod testimonials;
pub mod cta_final;

use leptos::prelude::*;
use crate::app::navigate;
use crate::app::Route;

#[component]
pub fn LandingPage() -> impl IntoView {
    view! {
        <div style="background: var(--tertiary); color: var(--secondary); min-height: 100vh;">
            // 1. Promo bar
            <div class="anim-fade-in" style="background: #111; border-bottom: 1px solid #1f1f1f; padding: 8px 0;">
                <div style="max-width: 1200px; margin: 0 auto; padding: 0 24px;">
                    <span class="label-sm" style="color: var(--primary-60);">
                        "◆ ARC · Now in alpha · git push to deploy"
                    </span>
                </div>
            </div>

            // 2. Top nav
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

            // 3. Hero
            {hero::Hero()}

            // 4. Features (with icons)
            {features::Features()}

            // 5. How it works (with icons)
            {how_it_works::HowItWorks()}

            // 6. Code block
            {code_block::CodeBlock()}

            // 7. Stats strip
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

            // 8. Comparison table
            {comparison::Comparison()}

            // 9. Testimonials
            {testimonials::Testimonials()}

            // 10. Pricing
            {pricing::Pricing()}

            // 11. FAQ
            {faq::Faq()}

            // 12. Final CTA
            {cta_final::CtaFinal()}

            // 13. Footer
            {footer::Footer()}
        </div>
    }
}
