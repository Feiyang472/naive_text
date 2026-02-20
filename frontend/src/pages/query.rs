use std::collections::HashMap;

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

#[derive(Clone)]
struct CloudItem {
    label: String,
    count: usize,
    size_px: f32,
    hue: u16,
    left_pct: f32,
    top_pct: f32,
    duration_s: f32,
    delay_s: f32,
}

fn seeded_hash(seed: &str) -> u64 {
    seed.bytes().fold(0_u64, |acc, b| {
        acc.wrapping_mul(131).wrapping_add(u64::from(b))
    })
}

fn pseudo_pick(seed: &str) -> bool {
    seeded_hash(seed) % 3 == 0
}

fn build_cloud_items(events: &[Event]) -> Vec<CloudItem> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for event in events {
        *counts.entry(event.person_name().to_string()).or_insert(0) += 1;
        let place_name = match &event.kind {
            crate::types::EventKind::Appointment { place, .. }
            | crate::types::EventKind::Promotion { place, .. } => {
                place.as_ref().map(|p| p.name.clone())
            }
            crate::types::EventKind::Battle { target_place, .. } => {
                target_place.as_ref().map(|p| p.name.clone())
            }
            _ => None,
        };
        if let Some(place) = place_name {
            *counts.entry(place).or_insert(0) += 1;
        }
    }

    let max_count = counts.values().copied().max().unwrap_or(1) as f32;
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    pairs
        .into_iter()
        .take(180)
        .filter(|(label, _)| pseudo_pick(label))
        .take(48)
        .enumerate()
        .map(|(i, (label, count))| {
            let weight = (count as f32 / max_count).sqrt();
            let size_px = (12.0 + weight * 26.0).min(34.0);
            let hash = seeded_hash(&label).wrapping_add((count * 17 + i) as u64);
            let left_pct = 4.0 + ((hash % 90) as f32);
            let top_pct = 8.0 + (((hash / 97) % 78) as f32);
            let duration_s = 6.2 + ((hash % 27) as f32 / 6.0);
            let delay_s = -((hash % 60) as f32 / 10.0);
            let hue = 182 + ((i * 17) % 96) as u16;

            CloudItem {
                label,
                count,
                size_px,
                hue,
                left_pct,
                top_pct,
                duration_s,
                delay_s,
            }
        })
        .collect()
}

#[component]
pub fn QueryPage() -> impl IntoView {
    let events_data: RwSignal<Option<Result<EventsJson, String>>> = RwSignal::new(None);
    let person_input = RwSignal::new(String::new());
    let place_input = RwSignal::new(String::new());
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
        person_input.set(String::new());
        place_input.set(String::new());
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
                    <button type="button" on:click=on_reset class="secondary">"重置"</button>
                </div>
            </form>

            {move || {
                match events_data.get() {
                    None => view! { <p class="loading">"数据加载中…"</p> }.into_any(),
                    Some(Err(e)) => view! { <p class="error">{e}</p> }.into_any(),
                    Some(Ok(ej)) => {
                        let person = person_input.get_untracked().trim().to_string();
                        let place = place_input.get_untracked().trim().to_string();
                        let era = era_input.get_untracked();
                        let from: Option<i32> = year_from.get_untracked().trim().parse().ok();
                        let to: Option<i32> = year_to.get_untracked().trim().parse().ok();
                        let kind = kind_filter.get_untracked();
                        let show_cloud = person.is_empty() && place.is_empty();

                        let cloud = if show_cloud {
                            let merged: Vec<Event> = ej
                                .events
                                .iter()
                                .chain(ej.unstructured_events.iter())
                                .cloned()
                                .collect();
                            let cloud_items = build_cloud_items(&merged);

                            view! {
                                <div class="cloud-panel">
                                    <p class="empty">"未指定人物 / 地点：显示随机高频词浮动框（字号按频次缩放并设上限）。"</p>
                                    <div class="word-cloud">
                                        {cloud_items.into_iter().map(|item| {
                                            let style = format!(
                                                "font-size:{:.1}px;--box-h:{};left:{:.1}%;top:{:.1}%;animation-duration:{:.2}s;animation-delay:{:.2}s;",
                                                item.size_px,
                                                item.hue,
                                                item.left_pct,
                                                item.top_pct,
                                                item.duration_s,
                                                item.delay_s,
                                            );
                                            view! {
                                                <span class="cloud-item" style=style>
                                                    {item.label}
                                                    <small>{item.count}</small>
                                                </span>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <span/> }.into_any()
                        };

                        if !submitted.get() {
                            return view! {
                                <div>
                                    {cloud}
                                    <p class="empty">"设置条件后点击检索。"</p>
                                </div>
                            }.into_any();
                        }

                        let mut matched: Vec<Event> = ej
                            .events
                            .iter()
                            .chain(ej.unstructured_events.iter())
                            .filter(|e| {
                                let person_ok = person.is_empty() || e.person_name().contains(&person);
                                let place_name = match &e.kind {
                                    crate::types::EventKind::Appointment { place, .. }
                                    | crate::types::EventKind::Promotion { place, .. } => {
                                        place.as_ref().map(|p| p.name.as_str())
                                    },
                                    crate::types::EventKind::Battle { target_place, .. } => {
                                        target_place.as_ref().map(|p| p.name.as_str())
                                    },
                                    _ => None,
                                };
                                let place_ok = place.is_empty()
                                    || place_name
                                        .map(|p| p.contains(&place))
                                        .unwrap_or(false);
                                let era_ok = era.is_empty()
                                    || e.time
                                        .as_ref()
                                        .map(|t| t.raw.contains(&era))
                                        .unwrap_or(false);
                                let year_ok = match (from, to, e.time.as_ref()) {
                                    (Some(f), Some(t), Some(tm)) => tm.year >= f as u8 && tm.year <= t as u8,
                                    (Some(f), None, Some(tm)) => tm.year >= f as u8,
                                    (None, Some(t), Some(tm)) => tm.year <= t as u8,
                                    (None, None, _) => true,
                                    _ => false,
                                };
                                let has_filter = !era.is_empty()
                                    || from.is_some()
                                    || to.is_some()
                                    || !person.is_empty()
                                    || !place.is_empty();
                                let kind_ok = kind == "all" || {
                                    match &e.kind {
                                        crate::types::EventKind::Appointment { .. } => kind == "任命",
                                        crate::types::EventKind::Promotion { .. } => kind == "晋升",
                                        crate::types::EventKind::Accession { .. } => kind == "即位",
                                        crate::types::EventKind::Battle { .. } => kind == "战役",
                                        crate::types::EventKind::Death { .. } => kind == "薨卒",
                                    }
                                };
                                (if has_filter {
                                    person_ok && place_ok && (era_ok || year_ok)
                                } else {
                                    true
                                }) && kind_ok
                            })
                            .cloned()
                            .collect();

                        matched.sort_by_key(|e| e.time.as_ref().map(|t| t.year).unwrap_or(255));
                        let count = matched.len();

                        if count == 0 {
                            return view! {
                                <div>
                                    {cloud}
                                    <p class="empty">"未找到符合条件的事件。"</p>
                                </div>
                            }.into_any();
                        }

                        view! {
                            <div>
                                {cloud}
                                <p class="query-description">
                                    "共找到 " <strong>{count}</strong> " 条事件"
                                </p>
                                <ul class="event-list">
                                    {matched.into_iter().take(500).map(|ev| {
                                        let kind_zh = ev.kind_zh();
                                        let person_name = ev.person_name().to_string();
                                        let time_str = ev.time.as_ref().map(|t| t.raw.clone());
                                        let place_name = match &ev.kind {
                                            crate::types::EventKind::Appointment { place, .. }
                                            | crate::types::EventKind::Promotion { place, .. } => {
                                                place.as_ref().map(|p| p.name.clone())
                                            },
                                            crate::types::EventKind::Battle { target_place, .. } => {
                                                target_place.as_ref().map(|p| p.name.clone())
                                            },
                                            _ => None,
                                        };
                                        view! {
                                            <li class="event-item">
                                                <div class="event-meta">
                                                    <span class="event-type-badge">{kind_zh}</span>
                                                    <span class="event-person">{person_name}</span>
                                                    {time_str.map(|t| view! { <span class="event-time">{t}</span> })}
                                                    {place_name.map(|p| view! { <span class="event-place">"📍 " {p}</span> })}
                                                </div>
                                                <div class="event-context">{ev.context.clone()}</div>
                                                <div class="event-source">{ev.source_file.clone()}</div>
                                            </li>
                                        }
                                    }).collect_view()}
                                    {if count > 500 {
                                        view! {
                                            <p style="color:#94a3b8;font-size:0.85rem;padding:0.5rem 0;">
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
