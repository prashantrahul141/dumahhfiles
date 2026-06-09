use crate::utils::env_or;
use lazy_static::lazy_static;
use serde::Deserialize;
use std::{fmt::Debug, path::PathBuf};
use tracing::{debug, info};
use yt_dlp::{Downloader, client::Libraries};

#[derive(Debug)]
pub struct RunTimeState {
    pub on_disk_files_size: usize,
    pub downloader: Downloader,
}

impl RunTimeState {
    pub async fn new() -> Self {
        info!("setting up new runttimestate");
        let libraries = Libraries::new(PathBuf::from("yt-dlp"), PathBuf::from("ffmpeg"));
        let downloader = Downloader::builder(libraries, "output")
            .add_arg("--no-playlist")
            .add_arg(format!("--max-filesize={}", CONFIG.max_file_size))
            .add_arg("--abort-on-error ")
            .build()
            .await
            .unwrap();
        debug!("yt-dlp args: {:?}", downloader.args());
        Self {
            on_disk_files_size: Default::default(),
            downloader,
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub root_dir: PathBuf,
    pub internal_host: String,
    pub internal_port: u16,
    pub external_protocol: String,
    pub external_host: String,
    pub max_filename_length: usize,
    pub max_on_disk_storage: usize,
    pub max_file_size: u64,
    pub retention_mins: f32,
    pub version: &'static str,
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct DownloadForm {
    pub url: String,
    pub password: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_dir: std::path::PathBuf::from(env_or("DUMAHH_ROOT_DIR", "files".to_string())),
            internal_host: env_or("DUMAHH_INTERNAL_HOST", "0.0.0.0".to_string()),
            internal_port: env_or("DUMAHH_INTERNAL_PORT", 3000),
            external_protocol: env_or("DUMAHH_EXTERNAL_PROTOCOL", "http".to_string()),
            external_host: env_or("DUMAHH_EXTERNAL_HOST", "0.0.0.0:3000".to_string()),
            max_filename_length: env_or("DUMAHH_MAX_FILENAME_LENGTH", 240),
            max_on_disk_storage: env_or("DUMAHH_MAX_ON_DISK_STORAGE", 5 * 1024 * 1024 * 1024),
            max_file_size: env_or("DUMAHH_MAX_FILE_SIZE", 100 * 1024 * 1024),
            retention_mins: env_or("DUMAHH_RETENTION_MINS", 5.0),
            password: {
                let a = env_or("DUMAHH_PASSWORD", "".to_string());
                if a.is_empty() { None } else { Some(a) }
            },
            version: env!("GIT_HASH"),
        }
    }
}

lazy_static! {
    pub static ref CONFIG: Config = Config::default();
}

impl std::fmt::Debug for CONFIG {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}
