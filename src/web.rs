use askama::Template;
use axum::{http::StatusCode, response::Html};

use crate::PUBLIC_MODE;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    public_mode: bool,
}

pub async fn index() -> Result<Html<String>, StatusCode> {
    let template = IndexTemplate {
        public_mode: PUBLIC_MODE,
    };
    let rendered = template
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(rendered))
}
