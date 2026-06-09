use std::sync::Arc;

use crate::state::{CONFIG, DownloadForm, RunTimeState};
use crate::template::{HtmlTemplate, IndexTemplate};
use crate::utils::{DumAhhError, clean_filename, limit_filename_len, parse_url};
use axum::Form;
use axum::body::Body;
use axum::http::header::CONTENT_DISPOSITION;
use axum::http::{HeaderValue, Request};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use tokio::sync::RwLock;
use tower::ServiceExt;
use tracing::{debug, error};
use yt_dlp::DownloadStatus;
use yt_dlp::{self};

pub async fn root() -> impl IntoResponse {
    HtmlTemplate(IndexTemplate {
        domain: format!("{}://{}", CONFIG.external_protocol, CONFIG.external_host),
        password_enabled: CONFIG.password.is_some(),
        max_time_mins: CONFIG.max_retention_mns,
        min_time_mins: CONFIG.min_retention_mns,
        max_file_size_mb: CONFIG.max_file_size / (1024 * 1024),
    })
}

pub async fn download(
    State(state): State<Arc<RwLock<RunTimeState>>>,
    Form(download_form): Form<DownloadForm>,
) -> Result<impl IntoResponse, (StatusCode, DumAhhError)> {
    let parsed_url = parse_url(&download_form.url).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    debug!(parsed_url = parsed_url.as_str());
    let r_guard = state.read().await;

    /* if passwords enabled */
    if CONFIG.password.is_some() {
        debug!("password is set, verifying.");
        if CONFIG.password != download_form.password {
            return Err((StatusCode::UNAUTHORIZED, DumAhhError::WrongPassword));
        }
        debug!("password verified.");
    }

    /* total files limit */
    if r_guard.on_disk_files_size >= CONFIG.max_on_disk_storage {
        error!(
            "reached set disk limit on_disk = {on_disk} max = {max}",
            on_disk = r_guard.on_disk_files_size,
            max = CONFIG.max_on_disk_storage
        );
        return Err((StatusCode::INSUFFICIENT_STORAGE, DumAhhError::OutOfStorage));
    }

    debug!("fetching video info");
    let video_info = r_guard
        .downloader
        .fetch_video_infos(parsed_url)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                DumAhhError::Internal("Failed to fetch video info".to_string()),
            )
        })?;

    debug!("getting best format");
    let video_format = video_info.best_audio_video_format().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            DumAhhError::Internal("Failed to fetch video info".to_string()),
        )
    })?;

    debug!("first surety check for file size limit");
    if let Some(approx_size) = video_format.file_info.filesize_approx
        && approx_size >= CONFIG.max_file_size as i64
    {
        return Err((
            StatusCode::INSUFFICIENT_STORAGE,
            DumAhhError::FileTooBig(CONFIG.max_file_size),
        ));
    }

    debug!("getting format's url");
    let url = video_format.url().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            DumAhhError::Internal("Failed to fetch video info".to_string()),
        )
    })?;

    debug!("getting format's url");
    let manager = r_guard.downloader.download_manager();

    /* create filename */
    debug!("cleaning filename and path");
    let cleaned_filename = limit_filename_len(
        format!(
            "{}.{}",
            clean_filename(video_info.title.clone()),
            video_format.codec_info.video_ext
        ),
        CONFIG.max_filename_length,
    );
    let cleaned_filepath = CONFIG.root_dir.join(&cleaned_filename);
    debug!(
        "cleaned filename = {:?}, cleaned_filepath = {:?}",
        &cleaned_filename, cleaned_filepath
    );

    /* start download */
    debug!("adding download to queue");
    let download_id = manager.enqueue(url, &cleaned_filepath, None).await;

    /* make sure we dont go over the limit while downloading */
    debug!("starting to download with id = {}", download_id);

    /* hold for download to finish */
    let status = manager
        .wait_for_completion(download_id)
        .await
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, DumAhhError::Unknown))?;
    match status {
        DownloadStatus::Completed => {}
        _ => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, DumAhhError::Unknown));
        }
    }

    /* return file */
    let service = tower_http::services::ServeFile::new(&cleaned_filepath);
    let mut response = service
        .oneshot(Request::new(Body::empty()))
        .await
        .inspect_err(|e| error!("failed to serve file: {e}"))
        .map_err(|_| (StatusCode::NOT_FOUND, DumAhhError::FileNotFound))?;
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(r#"attachment; filename="{cleaned_filename}""#)).map_err(
            |_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    DumAhhError::Internal("Failed to server file".to_string()),
                )
            },
        )?,
    );
    Ok((StatusCode::OK, response))
}
