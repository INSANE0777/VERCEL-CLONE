use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    let sections = vec![
        ("Product", vec!["Features", "Pricing", "Changelog"]),
        ("Docs", vec!["Quickstart", "API Reference", "Guides"]),
        ("Company", vec!["About", "Blog", "Contact"]),
        ("Open Source", vec!["GitHub", "Contributing", "License"]),
    ];

    view! {
        <footer style="background: var(--tertiary); border-top: 1px solid #1f1f1f; padding: 52px 24px;">
            <div style="max-width: 1200px; margin: 0 auto;">
                <div style="display: grid; grid-template-columns: 1fr 2fr; gap: 52px; margin-bottom: 40px;">
                    <div>
                        <div class="label-lg" style="color: var(--secondary); font-size: 18px; font-weight: 600; margin-bottom: 12px;">
                            "◆ ARC"
                        </div>
                        <span class="label-sm" style="background: #1f1f1f; color: var(--primary-60); padding: 4px 10px;">
                            "Built in Rust"
                        </span>
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 24px;">
                        {sections.into_iter().map(|(heading, links)| view! {
                            <div>
                                <div class="label-sm" style="color: var(--primary-60); margin-bottom: 16px;">
                                    {heading.to_uppercase()}
                                </div>
                                {links.into_iter().map(|link| view! {
                                    <div style="margin-bottom: 8px;">
                                        <a class="body-sm" style="color: #9ca3af;" href="#">{link}</a>
                                    </div>
                                }).collect_view()}
                            </div>
                        }).collect_view()}
                    </div>
                </div>
                <div style="border-top: 1px solid #1f1f1f; padding-top: 24px;">
                    <span class="label-sm" style="color: #6b7280;">
                        "© 2025 ARC. Open source under MIT."
                    </span>
                </div>
            </div>
        </footer>
    }
}
