use leptos::prelude::*;
use crate::icons;

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
                        let icon = match i {
                            0 => view! {
                                <span style="display: flex; color: var(--primary); width: 24px; height: 24px;">
                                    {icons::IconGitBranch()}
                                </span>
                            }.into_any(),
                            1 => view! {
                                <span style="display: flex; color: var(--primary); width: 24px; height: 24px;">
                                    {icons::IconCode()}
                                </span>
                            }.into_any(),
                            2 => view! {
                                <span style="display: flex; color: var(--primary); width: 24px; height: 24px;">
                                    {icons::IconBox()}
                                </span>
                            }.into_any(),
                            _ => view! {
                                <span style="display: flex; color: var(--primary); width: 24px; height: 24px;">
                                    {icons::IconGlobe()}
                                </span>
                            }.into_any(),
                        };
                        view! {
                            <div class={format!("anim-fade-up {}", delay)}>
                                <div style="display: flex; align-items: center; gap: 10px; margin-bottom: 16px;">
                                    <span class="label-lg" style="color: var(--primary);">
                                        {num}
                                    </span>
                                    {icon}
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
