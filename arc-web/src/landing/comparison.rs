use leptos::prelude::*;
use leptos::either::Either;
use crate::icons;

#[component]
pub fn Comparison() -> impl IntoView {
    let features: Vec<(&str, bool, bool, bool, bool)> = vec![
        ("Open source",           true,  false, false, true),
        ("Self-hostable",         true,  false, false, true),
        ("Firecracker isolation", true,  true,  false, false),
        ("Rust-native",           true,  false, false, false),
        ("Edge middleware",       true,  true,  true,  false),
        ("Free unlimited deploys", true, false, false, true),
    ];

    view! {
        <section style="background: var(--tertiary); padding: 104px 24px;">
            <div style="max-width: 1000px; margin: 0 auto;">
                <h2 class="headline-lg anim-fade-up" style="color: var(--secondary); margin-bottom: 52px; text-align: center;">
                    "Why ARC."
                </h2>
                <div style="overflow-x: auto;">
                    <table style="width: 100%; border-collapse: collapse;">
                        <thead>
                            <tr>
                                <th class="label-sm" style="text-align: left; padding: 16px 12px; color: #9ca3af; border-bottom: 1px solid #1f1f1f;">"FEATURE"</th>
                                <th class="label-sm" style="text-align: center; padding: 16px 12px; color: var(--primary); border-bottom: 2px solid var(--primary);">"ARC"</th>
                                <th class="label-sm" style="text-align: center; padding: 16px 12px; color: #9ca3af; border-bottom: 1px solid #1f1f1f;">"VERCEL"</th>
                                <th class="label-sm" style="text-align: center; padding: 16px 12px; color: #9ca3af; border-bottom: 1px solid #1f1f1f;">"NETLIFY"</th>
                                <th class="label-sm" style="text-align: center; padding: 16px 12px; color: #9ca3af; border-bottom: 1px solid #1f1f1f;">"SELF-HOST"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {features.into_iter().enumerate().map(|(i, (name, arc, vercel, netlify, selfhost))| {
                                let row_bg = if i % 2 == 0 { "background: #0f0f0f;" } else { "" };
                                view! {
                                    <tr style=row_bg>
                                        <td class="body-sm" style="padding: 16px 12px; color: var(--secondary); border-bottom: 1px solid #1f1f1f;">
                                            {name}
                                        </td>
                                        <td style="text-align: center; padding: 16px 12px; border-bottom: 1px solid #1f1f1f;">
                                            {if arc {
                                                Either::Left(view! {
                                                    <span style="display: inline-flex; color: var(--primary); width: 20px; height: 20px;">
                                                        {icons::IconCheck()}
                                                    </span>
                                                })
                                            } else {
                                                Either::Right(view! {
                                                    <span style="display: inline-flex; color: #4a4a4a; width: 20px; height: 20px;">
                                                        {icons::IconX()}
                                                    </span>
                                                })
                                            }}
                                        </td>
                                        <td style="text-align: center; padding: 16px 12px; border-bottom: 1px solid #1f1f1f;">
                                            {if vercel {
                                                Either::Left(view! {
                                                    <span style="display: inline-flex; color: var(--primary); width: 20px; height: 20px;">
                                                        {icons::IconCheck()}
                                                    </span>
                                                })
                                            } else {
                                                Either::Right(view! {
                                                    <span style="display: inline-flex; color: #4a4a4a; width: 20px; height: 20px;">
                                                        {icons::IconX()}
                                                    </span>
                                                })
                                            }}
                                        </td>
                                        <td style="text-align: center; padding: 16px 12px; border-bottom: 1px solid #1f1f1f;">
                                            {if netlify {
                                                Either::Left(view! {
                                                    <span style="display: inline-flex; color: var(--primary); width: 20px; height: 20px;">
                                                        {icons::IconCheck()}
                                                    </span>
                                                })
                                            } else {
                                                Either::Right(view! {
                                                    <span style="display: inline-flex; color: #4a4a4a; width: 20px; height: 20px;">
                                                        {icons::IconX()}
                                                    </span>
                                                })
                                            }}
                                        </td>
                                        <td style="text-align: center; padding: 16px 12px; border-bottom: 1px solid #1f1f1f;">
                                            {if selfhost {
                                                Either::Left(view! {
                                                    <span style="display: inline-flex; color: var(--primary); width: 20px; height: 20px;">
                                                        {icons::IconCheck()}
                                                    </span>
                                                })
                                            } else {
                                                Either::Right(view! {
                                                    <span style="display: inline-flex; color: #4a4a4a; width: 20px; height: 20px;">
                                                        {icons::IconX()}
                                                    </span>
                                                })
                                            }}
                                        </td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            </div>
        </section>
    }
}
