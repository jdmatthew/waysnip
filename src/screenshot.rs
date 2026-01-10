//! Screenshot capture functionality using grim

use gdk_pixbuf::Pixbuf;
use std::process::{Command, Stdio};

/// Error type for screenshot operations
#[derive(Debug)]
pub enum ScreenshotError {
    GrimNotFound,
    CaptureFailure(String),
    PixbufError(String),
}

impl std::fmt::Display for ScreenshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenshotError::GrimNotFound => {
                write!(f, "grim command not found. Please install grim.")
            }
            ScreenshotError::CaptureFailure(msg) => {
                write!(f, "Failed to capture screenshot: {}", msg)
            }
            ScreenshotError::PixbufError(msg) => write!(f, "Failed to load image: {}", msg),
        }
    }
}

impl std::error::Error for ScreenshotError {}

/// Captured screenshot data
pub struct Screenshot {
    /// Raw PNG bytes (kept for potential future use)
    #[allow(dead_code)]
    pub data: Vec<u8>,
    /// Loaded pixbuf for display
    pub pixbuf: Pixbuf,
    /// Screen width
    pub width: i32,
    /// Screen height
    pub height: i32,
}

impl Screenshot {
    /// Capture a screenshot using grim
    pub fn capture() -> Result<Self, ScreenshotError> {
        // Check if grim is available
        if Command::new("which")
            .arg("grim")
            .output()
            .map(|o| !o.status.success())
            .unwrap_or(true)
        {
            return Err(ScreenshotError::GrimNotFound);
        }

        // Execute grim to capture screenshot to stdout
        let output = Command::new("grim")
            .args(["-t", "png", "-"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| ScreenshotError::CaptureFailure(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ScreenshotError::CaptureFailure(stderr.to_string()));
        }

        let data = output.stdout;

        // Load into pixbuf using a memory input stream
        let bytes = glib::Bytes::from(&data);
        let stream = gtk4::gio::MemoryInputStream::from_bytes(&bytes);
        let pixbuf = Pixbuf::from_stream(&stream, gtk4::gio::Cancellable::NONE)
            .map_err(|e| ScreenshotError::PixbufError(e.to_string()))?;

        let width = pixbuf.width();
        let height = pixbuf.height();

        Ok(Screenshot {
            data,
            pixbuf,
            width,
            height,
        })
    }

    /// Crop the screenshot to the given rectangle
    pub fn crop(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<Vec<u8>, ScreenshotError> {
        // Clamp to valid bounds
        let x = x.max(0).min(self.width - 1);
        let y = y.max(0).min(self.height - 1);
        let width = width.min(self.width - x).max(1);
        let height = height.min(self.height - y).max(1);

        // Create a new subpixbuf for the selection
        let cropped = self.pixbuf.new_subpixbuf(x, y, width, height);

        // Save to PNG bytes
        cropped
            .save_to_bufferv("png", &[])
            .map_err(|e: glib::Error| ScreenshotError::PixbufError(e.to_string()))
    }
}
