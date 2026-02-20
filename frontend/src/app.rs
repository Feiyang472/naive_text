use leptos::prelude::*;
use leptos_router::{
    components::{A, Route, Router, Routes},
    path,
};

use crate::pages::{home::HomePage, person::PersonPage, query::QueryPage, timeline::TimelinePage};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div id="app">
                <header>
                    <h1>"六朝史书 · 事件检索"</h1>
                    <nav>
                        <A href="/">"首页"</A>
                        <A href="/person">"人物"</A>
                        <A href="/query">"时间"</A>
                        <A href="/timeline">"年号"</A>
                    </nav>
                </header>
                <main>
                    <Routes fallback=|| {
                        view! { <p class="error">"页面未找到"</p> }
                    }>
                        <Route path=path!("/") view=HomePage/>
                        <Route path=path!("/person") view=PersonPage/>
                        <Route path=path!("/query") view=QueryPage/>
                        <Route path=path!("/timeline") view=TimelinePage/>
                    </Routes>
                </main>
                <footer>
                    <p>"晋书 · 宋书 · 南齐书 · 梁书 · 陈书 · 魏书 — 六朝正史结构化提取"</p>
                </footer>
            </div>
        </Router>
    }
}
