use std::sync::Arc;
use std::time::Duration;

use crate::state::{CONFIG, DownloadForm, RunTimeState};
use crate::template::{HtmlTemplate, IndexTemplate};
use crate::utils::{DumAhhError, clean_file, clean_filename, limit_filename_len, parse_url};
use axum::Form;
use axum::body::Body;
use axum::http::header::CONTENT_DISPOSITION;
use axum::http::{HeaderValue, Request};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use tower::ServiceExt;
use tracing::{debug, error};

pub async fn root() -> impl IntoResponse {
    HtmlTemplate(IndexTemplate {
        domain: format!("{}://{}", CONFIG.external_protocol, CONFIG.external_host),
        password_enabled: CONFIG.password.is_some(),
        retention_mins: CONFIG.retention_mins,
        version: CONFIG.version,
    })
}

pub async fn download(
    State(state): State<Arc<RunTimeState>>,
    Form(download_form): Form<DownloadForm>,
) -> Result<impl IntoResponse, (StatusCode, DumAhhError)> {
    let parsed_url = parse_url(&download_form.url).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    debug!(parsed_url = parsed_url.as_str());

    /* if passwords enabled */
    if CONFIG.password.is_some() {
        debug!("password is set, verifying.");
        if CONFIG.password != download_form.password {
            return Err((StatusCode::UNAUTHORIZED, DumAhhError::WrongPassword));
        }
        debug!("password verified.");
    }

    /* total files limit */
    let ondisk = *state.on_disk_files_size.read().await;
    if ondisk >= CONFIG.max_on_disk_storage {
        error!(
            "reached set disk limit on_disk = {on_disk} max = {max}",
            on_disk = ondisk,
            max = CONFIG.max_on_disk_storage
        );
        return Err((StatusCode::INSUFFICIENT_STORAGE, DumAhhError::OutOfStorage));
    }

    /* if we can check file size now, check it */
    let mut was_filesize_updated = false;
    debug!("trying to check filesize");
    if let Ok(filesize) = state.downloader.get_filesize(&download_form.url).await {
        debug!("was able to retrieve file size = {}", filesize);
        was_filesize_updated = true;
        let ondisk = *state.on_disk_files_size.read().await;
        if ondisk + filesize >= CONFIG.max_on_disk_storage {
            error!(
                "reached storage limit ondisk + filesize = {}, max on disk = {}",
                ondisk + filesize,
                CONFIG.max_on_disk_storage
            );
            return Err((StatusCode::INSUFFICIENT_STORAGE, DumAhhError::OutOfStorage));
        }

        if filesize > CONFIG.max_file_size {
            error!(
                "file too big. filesize = {}, max_file_size = {}",
                filesize, CONFIG.max_file_size
            );
            return Err((StatusCode::INSUFFICIENT_STORAGE, DumAhhError::FileTooBig));
        }

        debug!("updating file size prematurely");
        *state.on_disk_files_size.write().await += filesize;
    }

    /* create filename */
    debug!("cleaning filename and path");
    let raw_filename = state
        .downloader
        .get_filename(&download_form.url)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let filename = limit_filename_len(clean_filename(raw_filename), CONFIG.max_filename_length);
    let filepath = CONFIG.root_dir.join(&filename);
    debug!("filename = {:?}, filepath = {:?}", &filename, &filepath);

    /* start download */
    debug!("adding download to queue");
    let _ = state
        .downloader
        .download(&download_form.url, &filename)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    debug!("download done, returing file path = {:?}", &filepath);

    /* create service to return file */
    let service = tower_http::services::ServeFile::new(&filepath);
    let mut response = service
        .oneshot(Request::new(Body::empty()))
        .await
        .inspect_err(|e| error!("failed to serve file: {e}"))
        .map_err(|_| (StatusCode::NOT_FOUND, DumAhhError::FileNotFound))?;
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(r#"attachment; filename="{}""#, filename))
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, DumAhhError::Internal))?,
    );

    /* update filesize if wasnt already */
    if !was_filesize_updated {
        debug!("filesize wasnt added, incrementing now.");
        let metadata = tokio::fs::metadata(&filepath)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, DumAhhError::Internal))?;
        *state.on_disk_files_size.write().await += metadata.len() as usize;
    }

    /* add cleanup before returning */
    tokio::spawn(async {
        let seconds = (CONFIG.retention_mins * 60.0) as u64;
        debug!("will delete {:?} after {} seconds", filepath, seconds);
        tokio::time::sleep(Duration::from_secs(seconds)).await;
        clean_file(filepath).await;
    });

    /* return file */
    Ok((StatusCode::OK, response))
}
