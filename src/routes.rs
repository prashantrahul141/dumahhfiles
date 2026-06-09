use crate::state::CONFIG;
use crate::template::{IndexTemplate, HtmlTemplate};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
};
use std::path;
use tokio::{fs };
use tracing::{error, warn};


pub async fn root() -> impl IntoResponse {
    HtmlTemplate(IndexTemplate {
        domain: format!("{}://{}", CONFIG.external_protocol, CONFIG.external_host),
        password_enabled: CONFIG.password.is_some(),
        max_time_mins: CONFIG.max_retention_mns,
        min_time_mins: CONFIG.min_retention_mns,
        max_file_size_mb: CONFIG.max_file_size / (1024 * 1024),
    })
}
