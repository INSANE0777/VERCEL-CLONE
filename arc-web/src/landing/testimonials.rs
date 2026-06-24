use leptos::prelude::*;

#[component]
pub fn Testimonials() -> impl IntoView {
    let items = vec![
        ("ARC replaced our entire CI/CD pipeline. Deployments that took 20 minutes now take 90 seconds.", "DevOps Engineer"),
        ("The framework auto-detection is magic. I just push and it works.", "Frontend Developer"),
        ("Running it on our own infrastructure saved us $400/mo in hosting costs.", "Startup CTO"),
    ];

    view! {
        <section style="background: var(--surface); padding: 104px 24px;">
            <div style="max-width: 1200px; margin: 0 auto;">
                <h2 class="headline-lg anim-fade-up" style="color: var(--on-surface); margin-bottom: 52px; text-align: center;">
                    "What developers say."
                </h2>
                <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 24px;">
                    {items.into_iter().enumerate().map(|(i, (quote, role))| {
                        let delay = format!("delay-{}", i + 1);
                        view! {
                            <div class={format!("card anim-fade-up {}", delay)} style="padding: 32px; display: flex; flex-direction: column; gap: 20px;">
                                <div style="font-family: var(--font-mono); font-size: 48px; color: var(--primary); line-height: 1;">"\""</div>
                                <p class="body-md" style="color: var(--on-surface); flex: 1;">
                                    {quote}
                                </p>
                                <div class="label-sm" style="color: var(--primary);">
                                    {format!("— {}", role)}
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </section>
    }
}
