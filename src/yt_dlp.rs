use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error};

use crate::{
    state::{CONFIG, YTDLP_ARGS, YTDLP_FILTER},
    utils::DumAhhError,
};

#[derive(Debug)]
pub struct DownloadCommand {
    pub prog: String,
    pub args: Vec<String>,
}

impl Default for DownloadCommand {
    fn default() -> Self {
        Self {
            prog: CONFIG.yt_dlp_path.to_owned(),
            args: Default::default(),
        }
    }
}

impl DownloadCommand {
    pub fn from_url(url: String, filename: String) -> Self {
        let mut s = DownloadCommand {
            args: YTDLP_ARGS.clone(),
            ..Default::default()
        };
        s.args.extend_from_slice(&YTDLP_FILTER);
        s.args.push("-o".into());
        s.args.push(filename);
        s.args.push(url);
        s
    }
}

#[derive(Debug, Default)]
pub struct Runner {}

impl Runner {
    pub async fn run(&self, cmd: &DownloadCommand) -> Result<std::process::Output, std::io::Error> {
        let mut proc = tokio::process::Command::new(&cmd.prog);
        debug!("running {} with args {:?}", cmd.prog, cmd.args);
        proc.args(&cmd.args);
        proc.output().await
    }
}

#[derive(Debug)]
pub struct DownloadOutput {
    // pub filepath: PathBuf,
}

#[derive(Debug)]
pub struct YtDlp {
    runner: Runner,
    permits: Arc<Semaphore>,
}

impl Default for YtDlp {
    fn default() -> Self {
        Self {
            runner: Default::default(),
            permits: Arc::new(Semaphore::new(CONFIG.concurrent_downlods)),
        }
    }
}

impl YtDlp {
    pub async fn download(&self, url: &str, filename: &str) -> Result<DownloadOutput, DumAhhError> {
        let cmd = DownloadCommand::from_url(url.to_owned(), filename.to_owned());
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|_| DumAhhError::DownloadFailed)?;

        let output = self
            .runner
            .run(&cmd)
            .await
            .inspect_err(|e| error!("failed to download: {}", e))
            .map_err(|_| DumAhhError::DownloadFailed)?;

        if !output.status.success() {
            error!(
                "stderr = {}",
                String::from_utf8(output.stderr).unwrap_or("".into())
            );
            return Err(DumAhhError::DownloadFailed);
        }

        Ok(DownloadOutput {})
    }

    pub async fn get_metadata(&self, url: &str) -> Result<Value, DumAhhError> {
        let mut cmd = DownloadCommand::default();
        cmd.args.push("--dump-json".into());
        cmd.args.extend_from_slice(&YTDLP_FILTER);
        cmd.args.push(url.into());

        let output = self
            .runner
            .run(&cmd)
            .await
            .inspect_err(|e| error!("failed to fetch info: {}", e))
            .map_err(|_| DumAhhError::DownloadFailed)?;

        if !output.status.success() {
            error!(
                "stderr = {}",
                String::from_utf8(output.stderr).unwrap_or("".into())
            );
            return Err(DumAhhError::DownloadFailed);
        }

        let out = String::from_utf8(output.stdout)
            .map_err(|_| DumAhhError::DownloadFailed)?
            .trim()
            .to_string();

        if out == "NA" {
            return Err(DumAhhError::DownloadFailed);
        }

        let metadata: Value = serde_json::from_str(&out).map_err(|_| {
            error!("failed to retrieve metadata");
            DumAhhError::Internal
        })?;

        Ok(metadata)
    }

    pub async fn get_media_type(&self, metadata: Value) -> Result<String, DumAhhError> {
        let m = match &metadata["media_type"] {
            Value::String(m) => m,
            _ => return Err(DumAhhError::Internal),
        };
        Ok(m.to_owned())
    }

    pub async fn get_filesize(&self, url: &str) -> Result<usize, DumAhhError> {
        if let Ok(s) = self.get_exact_filesize(url).await {
            debug!("got exact file size = {}", s);
            return Ok(s);
        }

        if let Ok(s) = self.get_approx_filesize(url).await {
            debug!("got approx file size = {}", s);
            return Ok(s);
        }

        debug!("failed to get filesize");
        Err(DumAhhError::DownloadFailed)
    }

    pub async fn get_approx_filesize(&self, url: &str) -> Result<usize, DumAhhError> {
        let s = self.get_property(url, "approx_filesize").await?;
        s.parse::<usize>().map_err(|_| DumAhhError::DownloadFailed)
    }

    pub async fn get_exact_filesize(&self, url: &str) -> Result<usize, DumAhhError> {
        let s = self.get_property(url, "filesize").await?;
        s.parse::<usize>().map_err(|_| DumAhhError::DownloadFailed)
    }

    pub async fn get_filename(&self, url: &str) -> Result<String, DumAhhError> {
        self.get_property(url, "filename").await
    }

    async fn get_property(&self, url: &str, property: &str) -> Result<String, DumAhhError> {
        let mut cmd = DownloadCommand::default();
        cmd.args.extend_from_slice(&YTDLP_FILTER);
        cmd.args
            .extend_from_slice(&["--print".into(), property.into(), url.to_owned()]);

        let output = self
            .runner
            .run(&cmd)
            .await
            .inspect_err(|e| error!("failed to fetch info: {}", e))
            .map_err(|_| DumAhhError::DownloadFailed)?;

        if !output.status.success() {
            error!(
                "stderr = {}",
                String::from_utf8(output.stderr).unwrap_or("".into())
            );
            return Err(DumAhhError::DownloadFailed);
        }
        let out = String::from_utf8(output.stdout)
            .map_err(|_| DumAhhError::DownloadFailed)?
            .trim()
            .to_string();

        if out == "NA" {
            return Err(DumAhhError::DownloadFailed);
        }

        Ok(out)
    }
}
