use crate::utils::env_or;
use lazy_static::lazy_static;
use std::{fmt::Debug, path::PathBuf};

#[derive(Debug)]
pub struct Config {
    pub root_dir: PathBuf,
    pub internal_host: String,
    pub internal_port: u16,
    pub external_protocol: String,
    pub external_host: String,
    pub max_file_count: usize,
    pub max_filename_length: usize,
    pub max_on_disk_storage: u64,
    pub max_file_size: usize,
    pub max_retention_mns: f32,
    pub min_retention_mns: f32,
    pub version: &'static str,
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
            max_file_count: env_or("DUMAHH_MAX_FILE_COUNT", 1),
            max_filename_length: env_or("DUMAHH_MAX_FILENAME_LENGTH", 240),
            max_on_disk_storage: env_or("DUMAHH_MAX_ON_DISK_STORAGE", 5 * 1024 * 1024 * 1024),
            max_file_size: env_or("DUMAHH_MAX_FILE_SIZE", 100 * 1024 * 1024),
            max_retention_mns: env_or("DUMAHH_MAX_RETENTION_MNS", 60.0),
            min_retention_mns: env_or("DUMAHH_MIN_RETENTION_MNS", 5.0),
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
