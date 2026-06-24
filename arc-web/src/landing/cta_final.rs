use leptos::prelude::*;
use crate::icons;

#[component]
pub fn CtaFinal() -> impl IntoView {
    let (email, set_email) = signal(String::new());

    view! {
        <section style="background: var(--tertiary); padding: 104px 24px; text-align: center;">
            <div style="max-width: 600px; margin: 0 auto;">
                <h2 class="headline-lg anim-fade-up" style="color: var(--secondary); margin-bottom: 16px;">
                    "Start deploying in minutes."
                </h2>
                <p class="body-md anim-fade-up delay-1" style="color: #9ca3af; margin-bottom: 40px;">
                    "Enter your email to get early access. No credit card required."
                </p>
                <div class="anim-fade-up delay-2" style="display: flex; gap: 12px; max-width: 480px; margin: 0 auto;">
                    <input class="form-input" type="email" placeholder="you@example.com"
                        style="flex: 1; background: #111; border: 1px solid #1f1f1f; color: var(--secondary); border-radius: 0;"
                        prop:value=email
                        on:input=move |ev| set_email.set(event_target_value(&ev)) />
                    <button class="btn btn-primary btn-sm"
                        on:click=move |_| {
                            if !email.get().is_empty() {
                                web_sys::window().unwrap()
                                    .alert_with_message("Thanks! We'll be in touch.")
                                    .unwrap_or(());
                                set_email.set(String::new());
                            }
                        }>
                        <span style="display: flex; width: 16px; height: 16px;">{icons::IconArrow()}</span>
                        "Get Early Access"
                    </button>
                </div>
            </div>
        </section>
    }
}
