use super::{html_escape, AppState};
use axum::{extract::{Path, State}, response::Html};

pub async fn index_page() -> Html<String> {
    Html(include_str!("../../templates/index.html").to_string())
}

pub async fn search_page() -> Html<String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let html = include_str!("../../templates/search.html")
        .replace("{{timestamp}}", &timestamp.to_string());
    Html(html)
}

pub async fn ai_page() -> Html<String> {
    Html(include_str!("../../templates/ai.html").to_string())
}

pub async fn compare_page() -> Html<String> {
    Html(include_str!("../../templates/compare.html").to_string())
}

pub async fn idea_page() -> Html<String> {
    Html(include_str!("../../templates/idea.html").to_string())
}

pub async fn settings_page() -> Html<String> {
    Html(include_str!("../../templates/settings.html").to_string())
}

pub async fn patent_detail_page(
    Path(id): Path<String>,
    State(s): State<AppState>,
) -> Html<String> {
    let t = include_str!("../../templates/patent_detail.html");
    match s.db.get_patent(&id) {
        Ok(Some(p)) => Html(
            t.replace("{{patent_number}}", &html_escape(&p.patent_number))
                .replace("{{title}}", &html_escape(&p.title))
                .replace("{{abstract_text}}", &html_escape(&p.abstract_text))
                .replace("{{description}}", &html_escape(&p.description))
                .replace("{{claims}}", &html_escape(&p.claims))
                .replace("{{applicant}}", &html_escape(&p.applicant))
                .replace("{{inventor}}", &html_escape(&p.inventor))
                .replace("{{filing_date}}", &html_escape(&p.filing_date))
                .replace("{{publication_date}}", &html_escape(&p.publication_date))
                .replace(
                    "{{grant_date}}",
                    &html_escape(&p.grant_date.unwrap_or_default()),
                )
                .replace("{{ipc_codes}}", &html_escape(&p.ipc_codes))
                .replace("{{cpc_codes}}", &html_escape(&p.cpc_codes))
                .replace("{{country}}", &html_escape(&p.country))
                .replace("{{legal_status}}", &html_escape(&p.legal_status))
                .replace("{{id}}", &html_escape(&p.id))
                .replace("{{images_json}}", &p.images)
                .replace("{{pdf_url}}", &html_escape(&p.pdf_url)),
        ),
        _ => Html("<h1>Not found</h1><a href='/search'>Back</a>".into()),
    }
}
