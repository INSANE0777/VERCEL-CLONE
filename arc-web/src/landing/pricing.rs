use leptos::prelude::*;
use crate::icons;

#[component]
pub fn Pricing() -> impl IntoView {
    let tiers = vec![
        ("Hobby", "Free", vec![
            "1 project",
            "10 deployments / mo",
            "Community support",
        ], false),
        ("Pro", "$20/mo", vec![
            "20 projects",
            "Unlimited deployments",
            "Custom domains",
            "Priority builds",
        ], true),
        ("Enterprise", "Custom", vec![
            "Unlimited everything",
            "SLA guarantee",
            "On-prem deployment",
            "Dedicated support",
        ], false),
    ];

    view! {
        <section style="background: var(--surface); padding: 104px 24px;">
            <div style="max-width: 1200px; margin: 0 auto;">
                <h2 class="headline-lg anim-fade-up" style="color: var(--on-surface); margin-bottom: 52px; text-align: center;">
                    "Simple pricing."
                </h2>
                <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 24px;">
                    {tiers.into_iter().enumerate().map(|(i, (name, price, features, highlighted))| {
                        let delay = format!("delay-{}", i + 1);
                        let border = if highlighted {
                            "2px solid var(--primary)".to_string()
                        } else {
                            "1px solid var(--border)".to_string()
                        };
                        view! {
                            <div class={format!("anim-fade-up {}", delay)}
                                style={format!("background: var(--surface); border: {}; border-radius: 8px; padding: 32px; display: flex; flex-direction: column;", border)}>
                                <div class="label-sm" style="color: var(--primary); margin-bottom: 8px;">
                                    {name.to_uppercase()}
                                </div>
                                <div class="headline-md" style="color: var(--on-surface); margin-bottom: 24px;">
                                    {price}
                                </div>
                                <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 32px; flex: 1;">
                                    {features.iter().map(|f| view! {
                                        <div style="display: flex; align-items: center; gap: 10px;">
                                            <span style="display: flex; color: var(--primary); width: 16px; height: 16px;">
                                                {icons::IconCheck()}
                                            </span>
                                            <span class="body-sm" style="color: var(--on-surface);">{*f}</span>
                                        </div>
                                    }).collect_view()}
                                </div>
                                <button class="btn btn-primary btn-sm"
                                    style={if highlighted { "" } else { "background: var(--tertiary); color: var(--secondary);" }}>
                                    "Get Started"
                                </button>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </section>
    }
}
