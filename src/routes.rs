use crate::state::{CONFIG, DownloadForm, RunTimeState};
use crate::template::{HtmlTemplate, IndexTemplate};
use crate::utils::{
    DumAhhError, clean_file, clean_filename, find_file_from_prefix, limit_filename_len, parse_url,
    split_name_ext,
};
use axum::Form;
use axum::body::Body;
use axum::http::header::CONTENT_DISPOSITION;
use axum::http::{HeaderValue, Request};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;
use tracing::{debug, error, info};

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
    info!(parsed_url = parsed_url.as_str());

    /* if passwords enabled */
    if CONFIG.password.is_some() {
        debug!("password is set, verifying.");
        if CONFIG.password != download_form.password {
            return Err((StatusCode::UNAUTHORIZED, DumAhhError::WrongPassword));
        }
        debug!("password verified.");
    }

    /* total files limit */
    info!("prelimary on disk size check");
    let ondisk = *state.on_disk_files_size.read().await;
    if ondisk >= CONFIG.max_on_disk_storage {
        error!(
            "reached set disk limit on_disk = {on_disk} max = {max}",
            on_disk = ondisk,
            max = CONFIG.max_on_disk_storage
        );
        return Err((StatusCode::INSUFFICIENT_STORAGE, DumAhhError::OutOfStorage));
    }

    let audio_only = download_form.audio_only.is_some();

    /* if we can check file size now, check it */
    let mut was_filesize_updated = false;
    let mut final_filesize = 0;
    info!("trying to check filesize");
    if let Ok(filesize) = state
        .downloader
        .get_filesize(&download_form.url, audio_only)
        .await
    {
        debug!("was able to retrieve file size = {}", filesize);
        was_filesize_updated = true;
        final_filesize = filesize;
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
    info!("cleaning filename and path");
    let raw_filename = state
        .downloader
        .get_filename(&download_form.url, audio_only)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let (file_prefix, file_ext) = split_name_ext(limit_filename_len(
        clean_filename(raw_filename),
        CONFIG.max_filename_length,
    ))
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let filepath = CONFIG.root_dir.join(&file_prefix);
    info!(
        "file_prefix = {:?} ext = {:?}, filepath = {:?}",
        &file_prefix, &file_ext, &filepath
    );

    /* start download */
    info!("adding download to queue");
    let _ = state
        .downloader
        .download(
            &download_form.url,
            &file_prefix,
            download_form.audio_only.is_some(),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    info!("download done, returing file path = {:?}", &filepath);

    /* create service to return file */
    let filepath = find_file_from_prefix(&CONFIG.root_dir, &file_prefix)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let service = tower_http::services::ServeFile::new(&filepath);
    let mut response = service
        .oneshot(Request::new(Body::empty()))
        .await
        .inspect_err(|e| error!("failed to serve file: {e}"))
        .map_err(|_| (StatusCode::NOT_FOUND, DumAhhError::FileNotFound))?;
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(r#"attachment; filename="{}""#, file_prefix)).map_err(
            |_| {
                error!("failed serving file");
                (StatusCode::INTERNAL_SERVER_ERROR, DumAhhError::Internal)
            },
        )?,
    );

    /* update filesize if wasnt already */
    if !was_filesize_updated {
        debug!("filesize wasnt added, incrementing now.");
        let metadata = tokio::fs::metadata(&filepath)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, DumAhhError::Internal))?;
        final_filesize = metadata.len() as usize;
        *state.on_disk_files_size.write().await += final_filesize;
    }

    /* add cleanup before returning */
    let copied_state = state.clone();
    tokio::spawn(async move {
        let seconds = (CONFIG.retention_mins * 60.0) as u64;
        debug!("will delete {:?} after {} seconds", filepath, seconds);
        tokio::time::sleep(Duration::from_secs(seconds)).await;
        *copied_state.on_disk_files_size.write().await -= final_filesize;
        clean_file(filepath).await;
    });

    /* return file */
    Ok((StatusCode::OK, response))
}
