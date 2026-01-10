//! Clipboard integration using wl-clipboard

use std::io::Write;
use std::process::{Command, Stdio};

/// Error type for clipboard operations
#[derive(Debug)]
pub enum ClipboardError {
    WlCopyNotFound,
    CopyFailure(String),
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipboardError::WlCopyNotFound => {
                write!(f, "wl-copy command not found. Please install wl-clipboard.")
            }
            ClipboardError::CopyFailure(msg) => write!(f, "Failed to copy to clipboard: {}", msg),
        }
    }
}

impl std::error::Error for ClipboardError {}

/// Check if wl-copy is available
pub fn is_wl_copy_available() -> bool {
    Command::new("which")
        .arg("wl-copy")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Copy PNG image data to clipboard
pub fn copy_image_to_clipboard(png_data: &[u8]) -> Result<(), ClipboardError> {
    if !is_wl_copy_available() {
        return Err(ClipboardError::WlCopyNotFound);
    }

    let mut child = Command::new("wl-copy")
        .args(["--type", "image/png"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ClipboardError::CopyFailure(e.to_string()))?;

    // Write PNG data to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(png_data)
            .map_err(|e| ClipboardError::CopyFailure(e.to_string()))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| ClipboardError::CopyFailure(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ClipboardError::CopyFailure(stderr.to_string()));
    }

    Ok(())
}
