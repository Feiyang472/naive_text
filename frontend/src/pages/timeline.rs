use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::types::TimelineJson;

async fn fetch_timeline() -> Result<TimelineJson, String> {
    let resp = gloo_net::http::Request::get("/data/timeline.json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json::<TimelineJson>().await.map_err(|e| e.to_string())
}

#[component]
pub fn TimelinePage() -> impl IntoView {
    let data: RwSignal<Option<Result<TimelineJson, String>>> = RwSignal::new(None);

    spawn_local(async move {
        data.set(Some(fetch_timeline().await));
    });

    view! {
        <div>
            <h2>"年号时间轴"</h2>
            <p style="color:#7a6e5f;font-size:0.9rem;margin-bottom:1rem;">
                "按政权和年号列出史书中出现的时间标记及其频次。"
            </p>
            {move || match data.get() {
                None => view! { <p class="loading">"加载中…"</p> }.into_any(),
                Some(Err(e)) => view! { <p class="error">{e}</p> }.into_any(),
                Some(Ok(tl)) => {
                    view! {
                        <div>
                            {tl.timeline.regimes.iter().map(|regime| {
                                let total: usize = regime.eras.iter()
                                    .map(|era| era.total_occurrences())
                                    .sum();
                                view! {
                                    <div class="regime-block card">
                                        <div class="regime-title">
                                            {regime.regime.clone()}
                                            <span style="font-weight:normal;font-size:0.8rem;color:#7a6e5f;margin-left:0.75rem;">
                                                "共 " {total} " 处"
                                            </span>
                                        </div>
                                        {regime.eras.iter().map(|era| {
                                            let occurrences = era.total_occurrences();
                                            let year_range = if era.years.is_empty() {
                                                String::new()
                                            } else {
                                                let first = &era.years[0];
                                                let last = era.years.last().unwrap();
                                                if first.year == last.year {
                                                    format!("{}年", first.year)
                                                } else {
                                                    format!("{}年—{}年", first.year, last.year)
                                                }
                                            };
                                            view! {
                                                <div class="era-row">
                                                    <span class="era-name">{era.era.clone()}</span>
                                                    <span class="era-years">{year_range}</span>
                                                    <span class="era-count">{occurrences} " 处"</span>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
