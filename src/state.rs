use crate::utils::env_or;
use crate::yt_dlp::YtDlp;
use lazy_static::lazy_static;
use serde::Deserialize;
use std::{fmt::Debug, path::PathBuf};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct RunTimeState {
    pub downloader: YtDlp,
    pub on_disk_files_size: RwLock<usize>,
}

impl RunTimeState {
    pub async fn new() -> Self {
        Self {
            downloader: YtDlp::default(),
            on_disk_files_size: Default::default(),
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
    pub max_file_size: usize,
    pub retention_mins: f32,
    pub concurrent_downlods: usize,
    pub version: &'static str,
    pub requests_per_minute: u64,
    pub yt_dlp_path: String,
    pub password: Option<String>,
    pub cookies_file: Option<PathBuf>,
}

#[derive(Deserialize)]
pub struct DownloadForm {
    pub url: String,
    pub password: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_dir: std::path::PathBuf::from(env_or("DUMAHH_ROOT_DIR", "./files".to_string())),
            internal_host: env_or("DUMAHH_INTERNAL_HOST", "0.0.0.0".to_string()),
            internal_port: env_or("DUMAHH_INTERNAL_PORT", 3000),
            external_protocol: env_or("DUMAHH_EXTERNAL_PROTOCOL", "http".to_string()),
            external_host: env_or("DUMAHH_EXTERNAL_HOST", "0.0.0.0:3000".to_string()),
            max_filename_length: env_or("DUMAHH_MAX_FILENAME_LENGTH", 240),
            max_on_disk_storage: env_or("DUMAHH_MAX_ON_DISK_STORAGE", 5 * 1024 * 1024 * 1024),
            max_file_size: env_or("DUMAHH_MAX_FILE_SIZE", 100 * 1024 * 1024),
            retention_mins: env_or("DUMAHH_RETENTION_MINS", 3.0),
            concurrent_downlods: env_or("DUMAHH_CONCURRENT_DOWNLOAD", 3),
            requests_per_minute: env_or("DUMAHH_REQUESTS_PER_MINUTE", 30),
            password: {
                let p = env_or("DUMAHH_PASSWORD", "".to_string());
                if p.is_empty() { None } else { Some(p) }
            },
            yt_dlp_path: env_or("DUMAHH_YTDLP_PATH", "yt-dlp".to_string()),
            cookies_file: {
                let p = env_or("DUMAHH_COOKIES_FILEPATH", "".to_string());
                if p.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(p))
                }
            },
            version: env!("GIT_HASH"),
        }
    }
}

lazy_static! {
    pub static ref CONFIG: Config = Config::default();
    pub static ref YTDLP_COOKIES: Vec<String> = {
        let mut args = vec![];
        if let Some(cookies_file) = &CONFIG.cookies_file {
            let cookies_file = cookies_file.clone().into_os_string().into_string().unwrap();
            args.extend_from_slice(&["--cookies".into(), cookies_file]);
        }
        args
    };
    pub static ref YTDLP_FILTER: Vec<String> = vec![
        "--match-filters".into(),
        "!is_live".into(),
        "--no-playlist".into(),
        "--max-filesize".into(),
        CONFIG.max_file_size.to_string(),
    ];
    pub static ref YTDLP_ARGS: Vec<String> = vec![
        "--no-exec".into(),
        "--abort-on-error".into(),
        "-P".into(),
        CONFIG
            .root_dir
            .clone()
            .into_os_string()
            .into_string()
            .unwrap(),
    ];
}

impl std::fmt::Debug for CONFIG {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}
