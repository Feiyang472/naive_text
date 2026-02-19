use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys;

use crate::types::{Event, EventsJson};

async fn fetch_events() -> Result<EventsJson, String> {
    let resp = gloo_net::http::Request::get("/data/events.json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json::<EventsJson>().await.map_err(|e| e.to_string())
}

#[component]
pub fn QueryPage() -> impl IntoView {
    let events_data: RwSignal<Option<Result<EventsJson, String>>> = RwSignal::new(None);
    let era_input = RwSignal::new(String::new());
    let year_from = RwSignal::new(String::new());
    let year_to = RwSignal::new(String::new());
    let kind_filter = RwSignal::new(String::from("all"));
    let submitted = RwSignal::new(false);

    spawn_local(async move {
        events_data.set(Some(fetch_events().await));
    });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        submitted.set(true);
    };

    let on_reset = move |_| {
        era_input.set(String::new());
        year_from.set(String::new());
        year_to.set(String::new());
        kind_filter.set("all".to_string());
        submitted.set(false);
    };

    view! {
        <div>
            <h2>"时间段检索"</h2>
            <p style="color:#7a6e5f;font-size:0.9rem;margin-bottom:0.75rem;">
                "按年号或公元年份筛选事件。年号示例：太和、元嘉、永明。"
            </p>
            <form on:submit=on_submit>
                <div class="search-row">
                    <input
                        type="text"
                        placeholder="年号（如：太和）"
                        prop:value=era_input
                        on:input=move |ev| {
                            era_input.set(event_target_value(&ev));
                            submitted.set(false);
                        }
                        style="max-width:200px;"
                    />
                    <input
                        type="text"
                        placeholder="AD 起始年（如：420）"
                        prop:value=year_from
                        on:input=move |ev| {
                            year_from.set(event_target_value(&ev));
                            submitted.set(false);
                        }
                        style="max-width:180px;"
                    />
                    <input
                        type="text"
                        placeholder="AD 结束年（如：479）"
                        prop:value=year_to
                        on:input=move |ev| {
                            year_to.set(event_target_value(&ev));
                            submitted.set(false);
                        }
                        style="max-width:180px;"
                    />
                    <select
                        on:change=move |ev| kind_filter.set(event_target_value(&ev))
                        style="max-width:120px;"
                    >
                        <option value="all">"全部类型"</option>
                        <option value="Appointment">"任命"</option>
                        <option value="Promotion">"晋升"</option>
                        <option value="Accession">"即位"</option>
                        <option value="Battle">"战役"</option>
                        <option value="Death">"薨卒"</option>
                    </select>
                    <button type="submit">"检索"</button>
                    <button type="button" on:click=on_reset
                        style="background:#aaa;">"重置"</button>
                </div>
            </form>

            {move || {
                if !submitted.get() {
                    return view! { <p class="empty">"设置条件后点击检索。"</p> }.into_any();
                }
                match events_data.get() {
                    None => view! { <p class="loading">"数据加载中…"</p> }.into_any(),
                    Some(Err(e)) => view! { <p class="error">{e}</p> }.into_any(),
                    Some(Ok(ej)) => {
                        let era = era_input.get_untracked();
                        let from: Option<i32> = year_from.get_untracked().trim().parse().ok();
                        let to: Option<i32> = year_to.get_untracked().trim().parse().ok();
                        let kind = kind_filter.get_untracked();

                        let mut matched: Vec<Event> = ej
                            .high_confidence
                            .iter()
                            .chain(ej.unstructured.iter())
                            .filter(|e| {
                                let d = e.data();
                                let era_ok = era.is_empty()
                                    || d.time
                                        .as_deref()
                                        .map(|t| t.contains(&era))
                                        .unwrap_or(false);
                                let year_ok = match (from, to, d.ad_year) {
                                    (Some(f), Some(t), Some(y)) => y >= f && y <= t,
                                    (Some(f), None, Some(y)) => y >= f,
                                    (None, Some(t), Some(y)) => y <= t,
                                    (None, None, _) => true,
                                    _ => false,
                                };
                                let has_filter = !era.is_empty() || from.is_some() || to.is_some();
                                let kind_ok = kind == "all" || e.kind_str() == kind;
                                (if has_filter { era_ok || year_ok } else { true }) && kind_ok
                            })
                            .cloned()
                            .collect();

                        matched.sort_by_key(|e| e.data().ad_year.unwrap_or(i32::MAX));

                        let count = matched.len();

                        if count == 0 {
                            return view! {
                                <p class="empty">"未找到符合条件的事件。"</p>
                            }.into_any();
                        }

                        view! {
                            <div>
                                <p style="color:#7a6e5f;font-size:0.88rem;margin-bottom:0.75rem;">
                                    "共找到 " <strong>{count}</strong> " 条事件"
                                </p>
                                <ul class="event-list">
                                    {matched.into_iter().take(500).map(|ev| {
                                        let d = ev.data().clone();
                                        let kind_str = ev.kind_str();
                                        let kind_zh = ev.kind_zh();
                                        view! {
                                            <li class=format!("event-item {kind_str}")>
                                                <div class="event-meta">
                                                    <span class="event-type-badge">{kind_zh}</span>
                                                    <span class="event-person">{d.person_name.clone()}</span>
                                                    {d.time.clone().map(|t| view! {
                                                        <span class="event-time">{t}</span>
                                                    })}
                                                    {d.place.clone().map(|p| view! {
                                                        <span class="event-place">"📍 " {p}</span>
                                                    })}
                                                    {d.ad_year.map(|y| view! {
                                                        <span class="event-time">"(AD " {y} ")"</span>
                                                    })}
                                                </div>
                                                <div class="event-context">{d.context}</div>
                                                <div class="event-source">{d.source_file}</div>
                                            </li>
                                        }
                                    }).collect_view()}
                                    {if count > 500 {
                                        view! {
                                            <p style="color:#999;font-size:0.85rem;padding:0.5rem 0;">
                                                "（仅显示前 500 条，请缩小检索范围）"
                                            </p>
                                        }.into_any()
                                    } else {
                                        view! { <span/> }.into_any()
                                    }}
                                </ul>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}
