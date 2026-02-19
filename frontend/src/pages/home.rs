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
pub fn HomePage() -> impl IntoView {
    let data: RwSignal<Option<Result<TimelineJson, String>>> = RwSignal::new(None);

    spawn_local(async move {
        data.set(Some(fetch_timeline().await));
    });

    view! {
        <div>
            <h2>"语料概览"</h2>
            {move || match data.get() {
                None => view! { <p class="loading">"加载中…"</p> }.into_any(),
                Some(Err(e)) => view! { <p class="error">{e}</p> }.into_any(),
                Some(Ok(tl)) => {
                    let s = &tl.stats;
                    view! {
                        <div class="stats-grid">
                            <div class="stat-card">
                                <div class="num">{s.total_events}</div>
                                <div class="label">"总事件数"</div>
                            </div>
                            <div class="stat-card">
                                <div class="num">{s.appointments}</div>
                                <div class="label">"任命"</div>
                            </div>
                            <div class="stat-card">
                                <div class="num">{s.promotions}</div>
                                <div class="label">"晋升"</div>
                            </div>
                            <div class="stat-card">
                                <div class="num">{s.accessions}</div>
                                <div class="label">"即位"</div>
                            </div>
                            <div class="stat-card">
                                <div class="num">{s.battles}</div>
                                <div class="label">"战役"</div>
                            </div>
                            <div class="stat-card">
                                <div class="num">{s.deaths}</div>
                                <div class="label">"薨卒"</div>
                            </div>
                            <div class="stat-card">
                                <div class="num">{s.unique_time_refs}</div>
                                <div class="label">"时间标记"</div>
                            </div>
                            <div class="stat-card">
                                <div class="num">{s.unique_places}</div>
                                <div class="label">"地名"</div>
                            </div>
                            <div class="stat-card">
                                <div class="num">{tl.timeline.total_time_points}</div>
                                <div class="label">"时间点"</div>
                            </div>
                        </div>
                        {if !s.top_places.is_empty() {
                            view! {
                                <div class="card">
                                    <h3>"高频地名"</h3>
                                    <ul style="list-style:none;display:flex;flex-wrap:wrap;gap:0.4rem;">
                                        {s.top_places.iter().take(20).map(|(name, count)| view! {
                                            <li style="background:#f4f0e8;border-radius:3px;padding:0.15rem 0.4rem;font-size:0.85rem;">
                                                {name.clone()}
                                                <span style="color:#999;margin-left:0.25rem;">{count.to_string()}</span>
                                            </li>
                                        }).collect_view()}
                                    </ul>
                                </div>
                            }.into_any()
                        } else {
                            view! { <span/> }.into_any()
                        }}
                    }.into_any()
                }
            }}
        </div>
    }
}
