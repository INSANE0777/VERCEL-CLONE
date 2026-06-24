use leptos::prelude::*;
use crate::icons;

#[component]
pub fn Faq() -> impl IntoView {
    let expanded = RwSignal::new(Option::<usize>::None);

    let items = vec![
        ("How does ARC detect my framework?",
         "We analyze your package.json dependencies and project structure. If Vite, Next.js, Astro, Remix, SvelteKit, Nuxt, CRA, or Gatsby is detected, the correct build command and output directory are inferred automatically."),
        ("Can I use custom domains?",
         "Yes, add custom domains in project settings. ARC configures Caddy edge routing automatically and provisions TLS certificates via Let's Encrypt."),
        ("What frameworks are supported?",
         "Vite, Next.js, Astro, Remix, SvelteKit, Nuxt, Create React App, and Gatsby. New frameworks are added regularly — the detection layer is extensible."),
        ("Is my data encrypted?",
         "Environment variables are encrypted at rest in PostgreSQL. Build artifacts are stored in encrypted S3-compatible storage. All internal traffic uses mTLS between services."),
        ("Can I self-host ARC?",
         "Yes, ARC is 100% open source under MIT. The entire stack — API server, builder, edge proxy, dashboard — runs on your own infrastructure with Docker Compose or Kubernetes."),
    ];

    view! {
        <section style="background: var(--tertiary); padding: 104px 24px;">
            <div style="max-width: 800px; margin: 0 auto;">
                <h2 class="headline-lg anim-fade-up" style="color: var(--secondary); margin-bottom: 52px; text-align: center;">
                    "FAQ"
                </h2>
                <div style="display: flex; flex-direction: column; gap: 12px;">
                    {items.into_iter().enumerate().map(|(i, (q, a))| {
                        let is_expanded = move || expanded.get() == Some(i);
                        view! {
                            <div style={move || {
                                let base = "border: 1px solid #1f1f1f; background: #111; transition: border-color 0.2s;";
                                if is_expanded() {
                                    format!("{} border-color: var(--primary);", base)
                                } else {
                                    base.to_string()
                                }
                            }}>
                                <div style="display: flex; align-items: center; justify-content: space-between; padding: 20px 24px; cursor: pointer;"
                                    on:click=move |_| {
                                        expanded.update(|e| {
                                            *e = if *e == Some(i) { None } else { Some(i) };
                                        });
                                    }>
                                    <span class="body-md" style="color: var(--secondary);">{q}</span>
                                    <span style={move || {
                                        let base = "display: flex; color: var(--primary-60); transition: transform 0.2s ease; width: 20px; height: 20px;";
                                        if is_expanded() {
                                            format!("{} transform: rotate(180deg);", base)
                                        } else {
                                            base.to_string()
                                        }
                                    }}>
                                        {icons::IconChevron()}
                                    </span>
                                </div>
                                {move || {
                                    if is_expanded() {
                                        view! {
                                            <div style="padding: 0 24px 20px; border-top: 1px solid #1f1f1f;">
                                                <p class="body-sm" style="color: #9ca3af; margin-top: 16px;">
                                                    {a}
                                                </p>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }
                                }}
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </section>
    }
}
