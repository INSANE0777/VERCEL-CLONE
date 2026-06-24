use leptos::prelude::*;

#[component]
pub fn CodeBlock() -> impl IntoView {
    let lines = vec![
        ("$ git push origin main", "prompt"),
        ("→ ARC detected: vite", "arrow"),
        ("→ Cloning repository...", "arrow"),
        ("→ Installing dependencies...", "arrow"),
        ("→ Building for production...", "arrow"),
        ("✓ Deployed to myapp.localhost", "success"),
    ];

    view! {
        <section style="background: var(--surface); padding: 104px 24px;">
            <div style="max-width: 800px; margin: 0 auto;">
                <h2 class="headline-lg" style="color: var(--on-surface); margin-bottom: 40px;">
                    "Zero config. Just push."
                </h2>
                <div class="terminal">
                    {lines.into_iter().map(|(line, kind)| view! {
                        <div>
                            <span class={
                                match kind {
                                    "prompt" => "terminal-prompt",
                                    "arrow" => "terminal-arrow",
                                    "success" => "terminal-success",
                                    _ => "",
                                }
                            }>{line}</span>
                        </div>
                    }).collect_view()}
                </div>
            </div>
        </section>
    }
}
