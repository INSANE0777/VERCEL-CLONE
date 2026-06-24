use leptos::prelude::*;

#[component]
pub fn HowItWorks() -> impl IntoView {
    let steps = vec![
        ("01", "Connect repo", "Link your GitHub repository. One click."),
        ("02", "Auto-detect framework", "Vite, Next.js, Astro, Remix, SvelteKit."),
        ("03", "Build in isolation", "Docker containers. Reproducible builds."),
        ("04", "Serve on edge", "Caddy reverse proxy. Atomic deploys."),
    ];

    view! {
        <section style="background: var(--tertiary); padding: 104px 24px;">
            <div style="max-width: 1200px; margin: 0 auto;">
                <h2 class="headline-lg anim-fade-up" style="color: var(--secondary); margin-bottom: 52px;">
                    "How it works."
                </h2>
                <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 24px;">
                    {steps.into_iter().enumerate().map(|(i, (num, title, desc))| {
                        let delay = format!("delay-{}", i + 1);
                        view! {
                            <div class={format!("anim-fade-up {}", delay)}>
                                <div class="label-lg" style="color: var(--primary); margin-bottom: 16px;">
                                    {num}
                                </div>
                                <h3 class="headline-sm" style="color: var(--secondary); margin-bottom: 8px;">
                                    {title}
                                </h3>
                                <p class="body-sm" style="color: #9ca3af;">
                                    {desc}
                                </p>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </section>
    }
}
