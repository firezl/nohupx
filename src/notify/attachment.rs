use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};

#[derive(Debug, Clone)]
pub struct LogAttachment {
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum AttachmentOutcome {
    Ready(LogAttachment),
    Skipped(String),
}

pub fn read_log_attachment(log_path: &Path, max_bytes: u64) -> Result<AttachmentOutcome> {
    if log_path.as_os_str().is_empty() {
        return Ok(AttachmentOutcome::Skipped(
            "log path is empty; attachment skipped".to_string(),
        ));
    }

    if !log_path.exists() {
        return Ok(AttachmentOutcome::Skipped(format!(
            "log file {} does not exist; attachment skipped",
            log_path.display()
        )));
    }

    let metadata = fs::metadata(log_path)
        .with_context(|| format!("failed to stat log file {}", log_path.display()))?;
    if metadata.len() > max_bytes {
        return Ok(AttachmentOutcome::Skipped(format!(
            "log file {} is {} bytes, exceeds attachment limit {} bytes; attachment skipped",
            log_path.display(),
            metadata.len(),
            max_bytes
        )));
    }

    let bytes = fs::read(log_path)
        .with_context(|| format!("failed to read log file {}", log_path.display()))?;
    Ok(AttachmentOutcome::Ready(LogAttachment {
        filename: log_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "nohupx.log".to_string()),
        bytes,
    }))
}

pub fn log_base64(attachment: &LogAttachment) -> String {
    STANDARD.encode(&attachment.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_missing_log_file() {
        let outcome =
            read_log_attachment(Path::new("/tmp/does-not-exist-nohupx.log"), 1024).unwrap();
        assert!(matches!(outcome, AttachmentOutcome::Skipped(_)));
    }

    #[test]
    fn reads_small_log_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("run.log");
        fs::write(&path, b"hello").unwrap();
        let outcome = read_log_attachment(&path, 1024).unwrap();
        match outcome {
            AttachmentOutcome::Ready(attachment) => {
                assert_eq!(attachment.filename, "run.log");
                assert_eq!(attachment.bytes, b"hello");
            }
            AttachmentOutcome::Skipped(reason) => panic!("unexpected skip: {reason}"),
        }
    }

    #[test]
    fn skips_when_file_too_large() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("big.log");
        fs::write(&path, vec![0_u8; 32]).unwrap();
        let outcome = read_log_attachment(&path, 16).unwrap();
        assert!(matches!(outcome, AttachmentOutcome::Skipped(_)));
    }
}
