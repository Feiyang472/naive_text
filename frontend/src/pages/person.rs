use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys;

use crate::types::{Event, EventKind, EventsJson};

async fn fetch_events() -> Result<EventsJson, String> {
    let resp = gloo_net::http::Request::get("/data/events.json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json::<EventsJson>().await.map_err(|e| e.to_string())
}

fn all_events(ej: &EventsJson) -> impl Iterator<Item = &Event> {
    ej.events.iter().chain(ej.unstructured_events.iter())
}

/// CJK character overlap score for name suggestion
fn name_similarity(candidate: &str, query: &str) -> usize {
    query.chars().filter(|c| candidate.contains(*c)).count()
}

#[component]
pub fn PersonPage() -> impl IntoView {
    let events_data: RwSignal<Option<Result<EventsJson, String>>> = RwSignal::new(None);
    let input = RwSignal::new(String::new());
    let submitted = RwSignal::new(String::new());

    spawn_local(async move {
        events_data.set(Some(fetch_events().await));
    });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        submitted.set(input.get_untracked());
    };

    view! {
        <div>
            <h2>"人物事件检索"</h2>
            <p style="color:#7a6e5f;font-size:0.9rem;margin-bottom:0.75rem;">
                "输入人名（如：陳顯達、褚淵、石勒），显示该人物在六朝史书中的全部事件记录。"
            </p>
            <form on:submit=on_submit>
                <div class="search-row">
                    <input
                        type="text"
                        placeholder="人名…"
                        prop:value=input
                        on:input=move |ev| input.set(event_target_value(&ev))
                    />
                    <button type="submit">"检索"</button>
                </div>
            </form>

            {move || {
                let query = submitted.get();
                if query.is_empty() {
                    return view! { <p class="empty">"请输入人名后点击检索。"</p> }.into_any();
                }
                match events_data.get() {
                    None => view! { <p class="loading">"数据加载中…"</p> }.into_any(),
                    Some(Err(e)) => view! { <p class="error">{e}</p> }.into_any(),
                    Some(Ok(ej)) => {
                        let mut matched: Vec<Event> = all_events(&ej)
                            .filter(|e| e.person_name() == query)
                            .cloned()
                            .collect();

                        if matched.is_empty() {
                            // Suggest similar names
                            let mut names: Vec<String> = all_events(&ej)
                                .map(|e| e.person_name().to_string())
                                .collect::<std::collections::HashSet<_>>()
                                .into_iter()
                                .collect();
                            names.sort_by_key(|n| std::cmp::Reverse(name_similarity(n, &query)));
                            let suggestions: Vec<String> = names
                                .into_iter()
                                .filter(|n| name_similarity(n, &query) > 0)
                                .take(8)
                                .collect();

                            return view! {
                                <div>
                                    <p class="empty">"未找到「" {query} "」的事件记录。"</p>
                                    {if !suggestions.is_empty() {
                                        view! {
                                            <p style="color:#7a6e5f;font-size:0.9rem;">"相似人名："</p>
                                            <ul style="list-style:none;display:flex;flex-wrap:wrap;gap:0.4rem;margin-top:0.4rem;">
                                                {suggestions.into_iter().map(|name| {
                                                    let name2 = name.clone();
                                                    view! {
                                                        <li>
                                                            <button
                                                                style="background:#f4f0e8;color:#1a1a1a;border:1px solid #d4c9b0;"
                                                                on:click=move |_| {
                                                                    input.set(name2.clone());
                                                                    submitted.set(name2.clone());
                                                                }
                                                            >{name}</button>
                                                        </li>
                                                    }
                                                }).collect_view()}
                                            </ul>
                                        }.into_any()
                                    } else {
                                        view! { <span/> }.into_any()
                                    }}
                                </div>
                            }.into_any();
                        }

                        // Sort by time year, untimed last
                        matched.sort_by_key(|e| e.time.as_ref().map(|t| t.year).unwrap_or(255));

                        let count = matched.len();
                        let first_year = matched.iter().find_map(|e| e.time.as_ref().map(|t| t.year as i32));
                        let last_year = matched.iter().rev().find_map(|e| e.time.as_ref().map(|t| t.year as i32));

                        view! {
                            <div>
                                <p style="color:#7a6e5f;font-size:0.88rem;margin-bottom:0.75rem;">
                                    "共找到 " <strong>{count}</strong> " 条事件"
                                    {match (first_year, last_year) {
                                        (Some(f), Some(l)) if f != l =>
                                            format!("，时间跨度 AD {} — AD {}", f, l),
                                        (Some(f), _) => format!("，约 AD {}", f),
                                        _ => String::new(),
                                    }}
                                </p>
                                <ul class="event-list">
                                    {matched.into_iter().map(|ev| {
                                        let kind_zh = ev.kind_zh();
                                        let time_str = ev.time.as_ref().map(|t| t.raw.clone());
                                        let place_str = match &ev.kind {
                                            EventKind::Appointment { place, .. } | EventKind::Promotion { place, .. } => {
                                                place.as_ref().map(|p| p.name.clone())
                                            },
                                            EventKind::Battle { target_place, .. } => {
                                                target_place.as_ref().map(|p| p.name.clone())
                                            },
                                            _ => None,
                                        };
                                        view! {
                                            <li class="event-item">
                                                <div class="event-meta">
                                                    <span class="event-type-badge">{kind_zh}</span>
                                                    {time_str.clone().map(|t| view! {
                                                        <span class="event-time">{t}</span>
                                                    })}
                                                    {place_str.map(|p| view! {
                                                        <span class="event-place">"📍 " {p}</span>
                                                    })}
                                                </div>
                                                <div class="event-context">{ev.context.clone()}</div>
                                                <div class="event-source">{ev.source_file.clone()}</div>
                                            </li>
                                        }
                                    }).collect_view()}
                                </ul>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}
