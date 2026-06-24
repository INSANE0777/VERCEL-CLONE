use leptos::prelude::*;

#[component]
pub fn CodeBlock() -> impl IntoView {
    let lines: Vec<(&'static str, &'static str)> = vec![
        ("$ git push origin main", "prompt"),
        ("→ ARC detected: vite", "arrow"),
        ("→ Cloning repository...", "arrow"),
        ("→ Installing dependencies...", "arrow"),
        ("→ Building for production...", "arrow"),
        ("✓ Deployed to myapp.localhost", "success"),
    ];
    let total = lines.len();

    view! {
        <section style="background: var(--surface); padding: 104px 24px;">
            <div style="max-width: 800px; margin: 0 auto;">
                <h2 class="headline-lg anim-fade-up" style="color: var(--on-surface); margin-bottom: 40px;">
                    "Zero config. Just push."
                </h2>
                <div class="terminal anim-scale-in delay-2">
                    {lines.into_iter().enumerate().map(|(i, (line, kind))| {
                        let delay = format!("delay-{}", i + 1);
                        let cls = match kind {
                            "prompt" => "terminal-prompt",
                            "arrow" => "terminal-arrow",
                            "success" => "terminal-success",
                            _ => "",
                        };
                        let is_last = i == total - 1;
                        view! {
                            <div class={format!("term-line {}", delay)}>
                                <span class={cls}>{line}</span>
                                {if is_last {
                                    view! { <span class="cursor-blink"></span> }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </section>
    }
}
