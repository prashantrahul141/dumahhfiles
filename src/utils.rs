use axum::response::IntoResponse;
use lazy_static::lazy_static;
use std::{ffi::OsString, io, path::PathBuf};
use thiserror::Error;
use tracing::{debug, error};
use url::Url;

#[derive(Error, Debug)]
#[allow(unused)]
pub enum DumAhhError {
    #[error("I/O Error\n")]
    IO(#[from] io::Error),
    #[error("Internal Error\n")]
    Internal,
    #[error("Invalid file name\n")]
    InvalidFileName,
    #[error("Failed to download")]
    DownloadFailed,
    #[error("Invalid Request: {0}\n")]
    InvalidRequest(String),
    #[error("Wrong password")]
    WrongPassword,
    #[error("Not a video")]
    NotAVideo,
    #[error("File too big\n")]
    FileTooBig,
    #[error("Too many files, allowed a maximum of: {0}\n")]
    TooManyFiles(usize),
    #[error("File not found\n")]
    FileNotFound,
    #[error("Storage bucket has reached its limit\n")]
    OutOfStorage,
    #[error("Unknown error\n")]
    Unknown,
}

impl IntoResponse for DumAhhError {
    fn into_response(self) -> axum::response::Response {
        self.to_string().into_response()
    }
}

use std::{env, ffi::OsStr, fmt::Debug, str::FromStr};

pub fn env_or<E, T>(key: E, default: T) -> T
where
    E: AsRef<OsStr>,
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    match env::var(key) {
        Ok(v) => v.parse::<T>().unwrap(),
        Err(_) => default,
    }
}

pub fn parse_url(s: &str) -> Result<Url, DumAhhError> {
    Url::parse(s.trim()).map_err(|_| DumAhhError::InvalidRequest("Invalid url".to_string()))
}

lazy_static! {
    // i really dont want random() to pick '.' as the first character
    static ref safe_chars: Vec<char> = ('a'..='z')
        .chain('A'..='Z')
        .chain('0'..='9')
        .chain('_'..='_')
        .collect::<Vec<char>>();
    static ref allowed_chars: Vec<char> = safe_chars
        .iter()
        .cloned()
        .chain(std::iter::once('.'))
        .collect();
}

pub fn clean_filename<S: AsRef<str>>(filename: S) -> String {
    filename
        .as_ref()
        .chars()
        .map(|x| if allowed_chars.contains(&x) { x } else { '_' })
        .collect::<String>()
}

fn truncate<S: AsRef<str>>(s: S, n: usize) -> String {
    s.as_ref().chars().take(n).collect()
}

pub fn split_name_ext<S: AsRef<str>>(filename: S) -> Result<(String, String), DumAhhError> {
    let filename = filename.as_ref().to_string();
    let pb = PathBuf::from(filename);

    let Some(ext) = pb.extension() else {
        error!("ext is none for pb = {pb:?}");
        Err(DumAhhError::InvalidFileName)?
    };

    let ext = ext
        .to_owned()
        .into_string()
        .map_err(|_| DumAhhError::InvalidFileName)?;

    let Some(prefix) = pb.file_prefix() else {
        error!("prefix is none for pb = {pb:?}");
        Err(DumAhhError::InvalidFileName)?
    };

    let prefix = prefix
        .to_owned()
        .into_string()
        .map_err(|_| DumAhhError::InvalidFileName)?;

    Ok((prefix, ext))
}

pub fn limit_filename_len<S: AsRef<str>>(filename: S, max_len: usize) -> String {
    let filename = filename.as_ref();

    if filename.chars().count() <= max_len {
        return filename.to_string();
    }

    let (stem, ext) = match filename.rsplit_once('.') {
        Some((stem, ext)) => (stem, Some(ext)),
        None => (filename, None),
    };

    match ext {
        Some(ext) => {
            let ext_len = ext.chars().count() + 1;

            if ext_len >= max_len {
                return format!(".{}", truncate(ext, max_len.saturating_sub(1)));
            }

            let allowed_stem_len = max_len - ext_len;

            format!("{}.{}", truncate(stem, allowed_stem_len), ext)
        }

        None => truncate(filename, max_len),
    }
}

pub async fn clean_file(path: PathBuf) {
    debug!("removing file = {:?}", path);
    _ = tokio::fs::remove_file(path).await;
}

pub fn find_file_from_prefix(
    dir_path: &PathBuf,
    target_name: &String,
) -> Result<PathBuf, DumAhhError> {
    // Read the contents of the directory
    let target_name = OsString::from(target_name);
    for entry in std::fs::read_dir(dir_path).map_err(|_| DumAhhError::FileNotFound)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_stem) = path.file_stem() {
                if file_stem == target_name {
                    return Ok(path);
                }
            }
        }
    }

    Err(DumAhhError::FileNotFound)
}
