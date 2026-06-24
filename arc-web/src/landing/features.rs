use leptos::prelude::*;

#[component]
pub fn Features() -> impl IntoView {
    let features = vec![
        ("Git-Pushed Deploys", "Push to deploy. Automatic PR previews. Zero config."),
        ("Framework Detection", "Vite, Next.js, Astro, Remix, SvelteKit — auto-detected."),
        ("Edge Routing", "Caddy-powered edge. Atomic deploys. Custom domains."),
    ];

    view! {
        <section style="background: var(--surface); padding: 104px 24px;">
            <div style="max-width: 1200px; margin: 0 auto;">
                <h2 class="headline-lg" style="color: var(--on-surface); margin-bottom: 52px;">
                    "Built for speed."
                </h2>
                <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 24px;">
                    {features.into_iter().map(|(title, desc)| view! {
                        <div class="card" style="padding: 32px;">
                            <div style="width: 32px; height: 32px; background: var(--primary); margin-bottom: 24px;"></div>
                            <h3 class="headline-sm" style="color: var(--on-surface); margin-bottom: 12px;">
                                {title}
                            </h3>
                            <p class="body-md" style="color: #6b7280;">
                                {desc}
                            </p>
                        </div>
                    }).collect_view()}
                </div>
            </div>
        </section>
    }
}
