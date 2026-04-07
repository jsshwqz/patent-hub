mod api;

use dioxus::prelude::*;

const STYLE: &str = include_str!("../assets/style.css");

fn main() {
    // Catch panics for debugging on mobile
    std::panic::set_hook(Box::new(|info| {
        eprintln!("PANIC: {info}");
    }));
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        style { {STYLE} }
        Router::<Route> {}
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Search {},
    #[route("/settings")]
    Settings {},
}

#[component]
fn Search() -> Element {
    let mut query = use_signal(|| String::new());
    let mut results: Signal<Vec<api::PatentSummary>> = use_signal(|| vec![]);
    let mut loading = use_signal(|| false);
    let mut total = use_signal(|| 0usize);
    let mut error_msg = use_signal(|| String::new());

    let do_search = move |_| {
        let q = query.read().clone();
        let srv = api::get_server_url();
        if q.is_empty() { return; }
        loading.set(true);
        error_msg.set(String::new());
        spawn(async move {
            match api::search_patents(&srv, &q, 1, 20).await {
                Ok((patents, t)) => {
                    results.set(patents);
                    total.set(t);
                }
                Err(e) => {
                    error_msg.set(format!("搜索失败: {e}"));
                    results.set(vec![]);
                }
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "app",
            nav { class: "navbar",
                span { class: "logo", "创研台" }
                Link { to: Route::Settings {}, class: "nav-link", "⚙️" }
            }
            div { class: "container",
                h2 { class: "section-title", "专利检索" }
                div { class: "search-bar",
                    input {
                        class: "search-input",
                        placeholder: "输入关键词、专利号、申请人...",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                    button {
                        class: "btn-primary",
                        onclick: do_search,
                        if *loading.read() { "搜索中..." } else { "搜索" }
                    }
                }
                if !error_msg.read().is_empty() {
                    div { class: "error-msg", "{error_msg}" }
                }
                if !results.read().is_empty() {
                    div { class: "results-header",
                        "找到 {total} 条"
                    }
                    for patent in results.read().iter() {
                        div { class: "patent-card",
                            div { class: "patent-number", "{patent.patent_number}" }
                            div { class: "patent-title", "{patent.title}" }
                            div { class: "patent-meta",
                                "{patent.applicant} | {patent.filing_date}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Settings() -> Element {
    let mut server_url = use_signal(|| api::get_server_url());
    let mut saved = use_signal(|| false);

    rsx! {
        div { class: "app",
            nav { class: "navbar",
                Link { to: Route::Search {}, class: "nav-link", "← 返回" }
                span { class: "logo", "设置" }
            }
            div { class: "container",
                h2 { class: "section-title", "服务器配置" }
                p { class: "hint", "输入 PC 上运行的创研台地址" }
                input {
                    class: "search-input",
                    placeholder: "http://192.168.1.100:3000",
                    value: "{server_url}",
                    oninput: move |e| {
                        server_url.set(e.value());
                        saved.set(false);
                    },
                }
                button {
                    class: "btn-primary",
                    onclick: move |_| {
                        api::set_server_url(&server_url.read());
                        saved.set(true);
                    },
                    "保存"
                }
                if *saved.read() {
                    div { class: "success-msg", "✅ 已保存" }
                }
                h2 { class: "section-title", "关于" }
                p { class: "hint", "创研台 InnoForge v0.5.0 · Dioxus Native" }
                p { class: "hint", "智能专利检索与分析平台" }
            }
        }
    }
}
